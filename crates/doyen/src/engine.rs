// SPDX-License-Identifier: MIT OR Apache-2.0
//! The election engine: the public object, the driver task, the lease, the schedule, the
//! snapshot and the subscribers.

use std::sync::Arc;

use futures_core::Stream;
use tokio::sync::oneshot;

use crate::backend::{self, Backend, BackendError};
use crate::clock::{Clock, SystemClock};
use crate::config::DatabaseConfig;
use crate::error::{Error, ErrorCode};
use crate::status::Status;
use crate::token::Token;

mod driver;
mod joining;
mod state;
mod stop;
mod wake_ups;

#[cfg(test)]
mod tests;

use driver::Driver;
use joining::{Connection, call_with_timeout};
use state::{Shared, left_error, log_failed_call};
use stop::StopSignals;

/// One instance of a service in a group. The oldest live instance of the group is the
/// master; the others are slaves.
///
/// Create it with [`Doyen::new`], then call [`join`](Doyen::join) once. Read
/// [`status`](Doyen::status) and [`token`](Doyen::token) at any time: they answer at once,
/// from the local lease. Call [`leave`](Doyen::leave) at shutdown, so the next instance
/// takes over at once.
///
/// Clones share one membership. When the last clone is dropped without `leave`, the object
/// stops at once (no more renews); its place expires after `timeout_ms`, and the other
/// instances take over as after a crash.
///
/// `join` and `leave` need a Tokio runtime; the renews run as a task on the runtime where
/// `join` was called.
///
/// In this version no database adapter is built in yet: `join` fails with
/// `DOYEN_UNSUPPORTED`. The Redis and MongoDB adapters come in the next versions.
///
/// ```no_run
/// use doyen::{DatabaseConfig, Doyen, RedisConfig, Status};
///
/// #[tokio::main(flavor = "current_thread")]
/// async fn main() -> Result<(), doyen::Error> {
///     let doyen = Doyen::new();
///     let config = DatabaseConfig::Redis(RedisConfig::new("redis://localhost", "billing-jobs"));
///     doyen.join(config).await?;
///
///     if doyen.status() == Status::Master {
///         if let Some(token) = doyen.token() {
///             println!("master, token {token:?}");
///         }
///     }
///
///     doyen.leave().await?;
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Doyen {
    handle: Arc<Handle>,
}

/// The part that the clones share. The driver never holds it, so the drop of the last
/// clone drops the stop sender, and the driver stops at once.
struct Handle {
    shared: Arc<Shared>,
    _stop_driver_on_drop: oneshot::Sender<()>,
}

impl Doyen {
    /// A new object, with no connection. Its status is offline `notJoined`.
    pub fn new() -> Doyen {
        Doyen::with_parts(Arc::new(SystemClock), None)
    }

    /// An object on the adapter and the clock of a test, in place of the adapter of the
    /// config and the real clock.
    #[cfg(test)]
    pub(crate) fn with_backend(backend: Arc<dyn Backend>, clock: Arc<dyn Clock>) -> Doyen {
        Doyen::with_parts(clock, Some(backend))
    }

    fn with_parts(clock: Arc<dyn Clock>, given_backend: Option<Arc<dyn Backend>>) -> Doyen {
        let (stop_sender, stop_receiver) = oneshot::channel();
        let shared = Shared::new(clock, given_backend, stop_receiver);
        let handle = Handle {
            shared: Arc::new(shared),
            _stop_driver_on_drop: stop_sender,
        };
        Doyen {
            handle: Arc::new(handle),
        }
    }

    /// The number of my place, while I have one.
    #[cfg(test)]
    pub(crate) fn number(&self) -> Option<u64> {
        self.handle.shared.number()
    }

    /// Join the group of the config: take a place at the end of the line.
    ///
    /// It finishes when the instance has a place (the status is then master or slave). It
    /// retries every network error meanwhile, with the status offline `connecting`. It
    /// fails only on an error a retry cannot fix: `DOYEN_INVALID_CONFIG`, `DOYEN_AUTH`,
    /// `DOYEN_UNSUPPORTED` (also outside a Tokio runtime), `DOYEN_ALREADY_JOINED` (a second
    /// `join`) and `DOYEN_LEFT` (after `leave`, or when `leave` is called meanwhile).
    ///
    /// Dropping the future (for example with `tokio::time::timeout`) cancels the join: the
    /// status goes back to `notJoined`, and a later `join` finds the place that the first
    /// one may have written.
    pub async fn join(&self, config: DatabaseConfig) -> Result<(), Error> {
        // `tokio::spawn` would panic outside a runtime (rust-api, "Tokio runtime").
        if tokio::runtime::Handle::try_current().is_err() {
            return Err(Error::new(
                ErrorCode::Unsupported,
                "join needs a Tokio runtime: call it from a task of a Tokio runtime",
            ));
        }
        let shared = &self.handle.shared;
        let connection = connection_for(shared, &config)?;
        shared.start_join(&connection)?;
        let _cancel_on_drop = CancelJoinOnDrop {
            shared: shared.clone(),
        };

        let mut signals = StopSignals::on_leave(shared.leaving());
        let joining = joining::first_join(shared, &connection);
        let (place, wake_ups) = match signals.unless_stopped(joining).await {
            None => return Err(left_error()),
            Some(Err(error)) => {
                shared.join_failed(&error);
                return Err(error);
            }
            Some(Ok(joined)) => joined,
        };
        if !shared.hold(place) {
            return Err(left_error());
        }
        if let Some(last_handle_dropped) = shared.take_driver_stop() {
            let signals = StopSignals::on_leave_or_drop(shared.leaving(), last_handle_dropped);
            Driver::start(shared.clone(), connection, signals, wake_ups);
        }
        Ok(())
    }

    /// Leave the group: stop being master at once, then remove the place, so the next
    /// member takes over without waiting for an expiry.
    ///
    /// The status is offline `left` at once, and stays so; this is the last notification.
    /// A pending `join` fails with `DOYEN_LEFT`. It finishes when the database removed the
    /// place, or after one call timeout (`renew_interval_ms`); a network error is not an
    /// error (the place then expires after `timeout_ms`). A second `leave` finishes at once.
    pub async fn leave(&self) -> Result<(), Error> {
        let shared = &self.handle.shared;
        let Some(place) = shared.start_leave() else {
            return Ok(());
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return Err(Error::new(
                ErrorCode::Unsupported,
                "leave needs a Tokio runtime to remove the place; the place expires after \
                 timeout_ms",
            ));
        }
        let connection = &place.connection;
        let removing = connection
            .backend
            .leave(connection.group(), &place.membership);
        match call_with_timeout(connection, removing).await {
            Ok(()) => Ok(()),
            Err(BackendError::Network(message)) => {
                log_failed_call(connection.group(), "leave", &message);
                Ok(())
            }
            // A refusal says what to fix, so `leave` returns it.
            Err(BackendError::Refused(error)) => Err(error),
        }
    }

    /// The status now: master, slave, or offline with a reason. It is computed from the
    /// local lease and the last known line at the moment of the call, and never waits.
    pub fn status(&self) -> Status {
        self.handle.shared.status()
    }

    /// The fencing token while the status is master, checked at the moment of the call;
    /// `None` otherwise.
    pub fn token(&self) -> Option<Token> {
        self.handle.shared.token()
    }

    /// A new stream of the status changes, from now on: each change of state or reason, in
    /// order, none lost. It does not start with the current status, and it ends after the
    /// `left` status.
    ///
    /// Each stream has its own queue with no limit: read it, or drop it.
    pub fn status_changes(&self) -> impl Stream<Item = Status> + Send + Unpin + 'static {
        let mut changes = self.handle.shared.subscribe();
        futures_util::stream::poll_fn(move |context| changes.poll_recv(context))
    }
}

impl Default for Doyen {
    fn default() -> Doyen {
        Doyen::new()
    }
}

/// The adapter and the checked config for `join`.
fn connection_for(shared: &Shared, config: &DatabaseConfig) -> Result<Connection, Error> {
    if let Some(backend) = &shared.given_backend {
        let settings = config.validate_settings()?;
        return Ok(Connection::new(backend.clone(), settings));
    }
    let settings = config.validate()?;
    let backend = backend::adapter_for(&settings)?;
    Ok(Connection::new(backend, settings))
}

/// Puts the status back to `notJoined` when a `join` future is dropped before it finished
/// (rust-api, "Cancel a join by dropping it"). After a place, a failure or a `leave` it
/// does nothing.
struct CancelJoinOnDrop {
    shared: Arc<Shared>,
}

impl Drop for CancelJoinOnDrop {
    fn drop(&mut self) {
        self.shared.cancel_join();
    }
}
