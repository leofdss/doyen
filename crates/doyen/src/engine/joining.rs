// SPDX-License-Identifier: MIT OR Apache-2.0
//! The calls that get a place: `connect`, `join` with its retries, and `renew`.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use super::state::Shared;
use super::wake_ups::WakeUps;
use crate::backend::{self, Backend, BackendError, Membership, View, Warning};
use crate::config::Validated;
use crate::error::Error;

/// What the engine needs to reach the database: the adapter and the checked config.
#[derive(Clone)]
pub(super) struct Connection {
    pub(super) backend: Arc<dyn Backend>,
    pub(super) settings: Arc<Validated>,
}

impl Connection {
    pub(super) fn new(backend: Arc<dyn Backend>, settings: Validated) -> Connection {
        Connection {
            backend,
            settings: Arc::new(settings),
        }
    }

    pub(super) fn group(&self) -> &str {
        &self.settings.group
    }

    /// The renew interval. It is also the timeout of each database call, so at most one
    /// renew runs at a time (election-core, "Renews at a fixed rate").
    pub(super) fn renew_interval(&self) -> Duration {
        self.settings.renew_interval
    }

    /// The local lease of a successful renew: `timeoutMs` − a margin of 10% (election-core,
    /// "Local lease").
    fn lease_length(&self) -> Duration {
        let timeout = self.settings.timeout;
        timeout - timeout / 10
    }
}

/// A place in the line: my membership, the last line I read and the end of my local lease
/// (on the clock that counts suspend).
pub(super) struct Place {
    pub(super) membership: Membership,
    pub(super) view: View,
    pub(super) lease_end: Duration,
}

impl Place {
    /// The master is the oldest live member: the first number of the line.
    pub(super) fn is_first_in_line(&self) -> bool {
        self.view.line.first() == Some(&self.membership.number)
    }
}

/// The answer of one renew.
pub(super) enum RenewOutcome {
    Renewed(Place),
    PlaceGone,
    Failed,
}

/// The delays between the retries: from 100 ms, × 2 each time, at most one renew interval,
/// with ± 20% jitter so the members of a group do not retry together.
pub(super) struct Backoff {
    next_delay: Duration,
    longest_delay: Duration,
}

impl Backoff {
    const FIRST_DELAY: Duration = Duration::from_millis(100);

    pub(super) fn new(longest_delay: Duration) -> Backoff {
        Backoff {
            next_delay: Backoff::FIRST_DELAY.min(longest_delay),
            longest_delay,
        }
    }

    pub(super) fn next_delay(&mut self) -> Duration {
        let delay = self.next_delay;
        self.next_delay = (delay * 2).min(self.longest_delay);
        let jitter: f64 = rand::random_range(0.8..=1.2);
        delay.mul_f64(jitter)
    }
}

/// Run one database call with the call timeout. A timeout is a network error: the request
/// may or may not have reached the database.
pub(super) async fn call_with_timeout<T>(
    connection: &Connection,
    call: impl Future<Output = backend::Result<T>>,
) -> backend::Result<T> {
    let timeout = connection.renew_interval();
    match tokio::time::timeout(timeout, call).await {
        Ok(answer) => answer,
        Err(_) => Err(BackendError::Network(format!(
            "the database did not answer within {} ms",
            timeout.as_millis()
        ))),
    }
}

/// The first `join` of an object: `connect` once, open the watch stream, then join until
/// there is a place. Network errors are retried; a refusal ends it (election-core, "Join
/// retries errors it can fix").
///
/// The watch stream opens before the join, so a wake-up just after the join (the master
/// leaves) is not lost.
pub(super) async fn first_join(
    shared: &Arc<Shared>,
    connection: &Connection,
) -> Result<(Place, WakeUps), Error> {
    let mut backoff = Backoff::new(connection.renew_interval());
    connect(shared, connection, &mut backoff).await?;
    let wake_ups = WakeUps::open(shared.clone(), connection.clone()).await;
    let place = join_until_placed(shared, connection, &mut backoff).await?;
    Ok((place, wake_ups))
}

async fn connect(
    shared: &Shared,
    connection: &Connection,
    backoff: &mut Backoff,
) -> Result<(), Error> {
    loop {
        match call_with_timeout(connection, connection.backend.connect()).await {
            Ok(warnings) => {
                log_warnings(connection.group(), &warnings);
                shared.send_warnings(&warnings);
                return Ok(());
            }
            Err(BackendError::Refused(error)) => return Err(error),
            Err(BackendError::Network(message)) => shared.call_failed("connect", &message),
        }
        tokio::time::sleep(backoff.next_delay()).await;
    }
}

/// Each warning once, at warn, with its code (status-reporting, "Warnings once at connect").
fn log_warnings(group: &str, warnings: &[Warning]) {
    for warning in warnings {
        tracing::warn!(
            target: "doyen",
            group,
            code = warning.code.as_str(),
            "{}",
            warning.message
        );
    }
}

/// Join with the kept tag until the database gives a place. Network errors are retried
/// with the backoff; a refusal is returned to the caller, which decides.
pub(super) async fn join_until_placed(
    shared: &Shared,
    connection: &Connection,
    backoff: &mut Backoff,
) -> Result<Place, Error> {
    loop {
        if let Some(place) = join_once(shared, connection).await? {
            return Ok(place);
        }
        tokio::time::sleep(backoff.next_delay()).await;
    }
}

/// One join attempt. `Ok(None)`: no place yet, try again.
async fn join_once(shared: &Shared, connection: &Connection) -> Result<Option<Place>, Error> {
    let settings = &connection.settings;
    let tag = shared.tag_for_attempt();
    let joining = connection.backend.join(
        &settings.group,
        &settings.instance_name,
        tag,
        settings.timeout,
    );
    let (number, view) = match call_with_timeout(connection, joining).await {
        Ok(answer) => answer,
        Err(BackendError::Network(message)) => {
            shared.call_failed("join", &message);
            return Ok(None);
        }
        Err(BackendError::Refused(error)) => return Err(error),
    };
    let membership = Membership { number, tag };
    if !view.line.contains(&number) {
        // The place died before the answer came: the next attempt is a new membership.
        shared.new_tag();
        return Ok(None);
    }

    // An earlier attempt with this tag may have written the place, so the lease counts from
    // the first attempt (election-core, "Lease of a join").
    let lease_end = shared.first_attempt_sent_at() + connection.lease_length();
    let time_left = lease_end.saturating_sub(shared.clock.now());
    if time_left >= connection.renew_interval() {
        let place = Place {
            membership,
            view,
            lease_end,
        };
        return Ok(Some(place));
    }
    // Less than one renew interval of that lease is left: renew at once.
    match renew(shared, connection, membership).await {
        RenewOutcome::Renewed(place) => Ok(Some(place)),
        RenewOutcome::PlaceGone => {
            shared.new_tag();
            Ok(None)
        }
        RenewOutcome::Failed => Ok(None),
    }
}

/// Keep my place alive and read the line. The new lease counts from the send time.
pub(super) async fn renew(
    shared: &Shared,
    connection: &Connection,
    membership: Membership,
) -> RenewOutcome {
    let settings = &connection.settings;
    let sent_at = shared.clock.now();
    let renewing = connection
        .backend
        .renew(&settings.group, &membership, settings.timeout);
    match call_with_timeout(connection, renewing).await {
        Ok(Some(view)) if view.line.contains(&membership.number) => RenewOutcome::Renewed(Place {
            membership,
            view,
            lease_end: sent_at + connection.lease_length(),
        }),
        // The place expired, has another tag, or is not in the line (election-core, "A lost
        // place means a new join at the end").
        Ok(_) => RenewOutcome::PlaceGone,
        Err(error) => {
            shared.call_failed("renew", &error.to_string());
            RenewOutcome::Failed
        }
    }
}
