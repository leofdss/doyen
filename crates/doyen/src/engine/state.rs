// SPDX-License-Identifier: MIT OR Apache-2.0
//! The snapshot of one object: its phase, its place, its lease, its last error and its
//! subscribers, behind one lock.
//!
//! `status()` and `token()` lock it for a moment and compute the answer with the clock at
//! the moment of the call; they never wait for a task or a database call (election-core,
//! "Status and token at the moment of the call").

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, watch};

use super::joining::{Connection, Place};
use crate::backend::{Backend, Membership, Warning};
use crate::clock::Clock;
use crate::error::{Error, ErrorCode};
use crate::status::{OfflineReason, Status};
use crate::token::Token;

/// A check of a binding: `false` means the app cannot work now (for example a blocked
/// Node.js event loop), so the driver skips the renew.
pub(super) type LivenessCheck = Arc<dyn Fn() -> bool + Send + Sync>;

/// The part of an object that the driver task shares with the handles.
pub(super) struct Shared {
    pub(super) clock: Arc<dyn Clock>,
    /// The adapter that a test gives (the in-memory one). `None` in an app: `join` takes the
    /// adapter of the config.
    pub(super) given_backend: Option<Arc<dyn Backend>>,
    state: Mutex<State>,
    /// Becomes `true` when `leave` starts. A pending join and the driver stop on it.
    leaving: watch::Sender<bool>,
}

/// Where an object is in its life. Only `Holding` has a place.
enum Phase {
    NotJoined,
    Connecting,
    Holding(Place),
    Reconnecting,
    Left,
}

struct State {
    phase: Phase,
    /// The group of the last `join`, for the log events.
    group: String,
    /// The tag of the next place. It is kept until a join returns a place, so a retry or a
    /// new `join` after a cancel finds the place that an earlier attempt may have written
    /// (election-core, "Membership tag").
    next_tag: u128,
    /// The send time of the first join attempt with `next_tag`: the lease of a join counts
    /// from it (election-core, "Lease of a join").
    first_attempt_sent_at: Option<Duration>,
    /// The message of the last failed database call, since the last call that worked.
    last_error: Option<String>,
    /// The last status sent to the subscribers. Only a new state or reason is sent.
    last_published: Status,
    /// One unbounded queue per stream: a change is never lost or merged.
    subscribers: Vec<mpsc::UnboundedSender<Status>>,
    /// One unbounded queue per warnings stream of a binding.
    warning_subscribers: Vec<mpsc::UnboundedSender<Warning>>,
    /// The liveness check of a binding. `None`: every renew is sent.
    liveness_check: Option<LivenessCheck>,
    /// The adapter and the config of the last `join`, for `leave`.
    connection: Option<Connection>,
    /// The other end of the handle's stop signal. The driver takes it when it starts.
    driver_stop: Option<oneshot::Receiver<()>>,
}

/// What `leave` must remove from the database.
pub(super) struct PlaceToRemove {
    pub(super) connection: Connection,
    pub(super) membership: Membership,
}

impl Shared {
    pub(super) fn new(
        clock: Arc<dyn Clock>,
        given_backend: Option<Arc<dyn Backend>>,
        driver_stop: oneshot::Receiver<()>,
    ) -> Shared {
        let not_joined = Status::Offline {
            reason: OfflineReason::NotJoined,
            error: None,
        };
        let state = State {
            phase: Phase::NotJoined,
            group: String::new(),
            next_tag: rand::random(),
            first_attempt_sent_at: None,
            last_error: None,
            last_published: not_joined,
            subscribers: Vec::new(),
            warning_subscribers: Vec::new(),
            liveness_check: None,
            connection: None,
            driver_stop: Some(driver_stop),
        };
        Shared {
            clock,
            given_backend,
            state: Mutex::new(state),
            leaving: watch::Sender::new(false),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A panic while the lock is held cannot leave the state half written (each change
        // is one assignment), so a poisoned lock is still safe to read. `status()` must
        // never panic.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The status now. A change that the driver did not send yet (the lease ended a moment
    /// ago) is sent first, so a subscriber never lags behind what `status()` returned.
    pub(super) fn status(&self) -> Status {
        let now = self.clock.now();
        self.lock().publish_change(now)
    }

    pub(super) fn token(&self) -> Option<Token> {
        let now = self.clock.now();
        let mut state = self.lock();
        state.publish_change(now);
        state.token(now)
    }

    /// The number of my place, while I have one.
    #[cfg(test)]
    pub(super) fn number(&self) -> Option<u64> {
        self.membership().map(|membership| membership.number)
    }

    pub(super) fn membership(&self) -> Option<Membership> {
        match &self.lock().phase {
            Phase::Holding(place) => Some(place.membership),
            _ => None,
        }
    }

    /// The Tokio time when the lease ends, while it runs. The Tokio timers do not count a
    /// suspend, so this is only a timer for the notification: `status()` reads the clock.
    pub(super) fn lease_deadline(&self) -> Option<tokio::time::Instant> {
        let now = self.clock.now();
        let lease_end = match &self.lock().phase {
            Phase::Holding(place) => place.lease_end,
            _ => return None,
        };
        let time_left = lease_end.checked_sub(now)?;
        if time_left.is_zero() {
            return None;
        }
        Some(tokio::time::Instant::now() + time_left)
    }

    /// A new queue of status changes. After `left` the queue is closed at once.
    pub(super) fn subscribe(&self) -> mpsc::UnboundedReceiver<Status> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut state = self.lock();
        if !matches!(state.phase, Phase::Left) {
            state.subscribers.push(sender);
        }
        receiver
    }

    /// A new queue of connect warnings, from now on. After `left` the queue is closed at
    /// once.
    pub(super) fn subscribe_to_warnings(&self) -> mpsc::UnboundedReceiver<Warning> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut state = self.lock();
        if !matches!(state.phase, Phase::Left) {
            state.warning_subscribers.push(sender);
        }
        receiver
    }

    /// Send each connect warning once to every warnings stream.
    pub(super) fn send_warnings(&self, warnings: &[Warning]) {
        let mut state = self.lock();
        for warning in warnings {
            state
                .warning_subscribers
                .retain(|subscriber| subscriber.send(warning.clone()).is_ok());
        }
    }

    pub(super) fn set_liveness_check(&self, check: LivenessCheck) {
        self.lock().liveness_check = Some(check);
    }

    /// `true` when the app can work now, or when no binding gave a check.
    pub(super) fn is_live(&self) -> bool {
        // The check is code of the binding: it runs without the lock, so it cannot block
        // `status()`.
        let check = self.lock().liveness_check.clone();
        match check {
            Some(check) => check(),
            None => true,
        }
    }

    pub(super) fn leaving(&self) -> watch::Receiver<bool> {
        self.leaving.subscribe()
    }

    pub(super) fn take_driver_stop(&self) -> Option<oneshot::Receiver<()>> {
        self.lock().driver_stop.take()
    }

    /// Start a `join`: check the one-life rules (election-core, "One object, one life").
    pub(super) fn start_join(&self, connection: &Connection) -> Result<(), Error> {
        let now = self.clock.now();
        let mut state = self.lock();
        match state.phase {
            Phase::NotJoined => {}
            Phase::Left => return Err(left_error()),
            Phase::Connecting | Phase::Holding(_) | Phase::Reconnecting => {
                return Err(Error::new(
                    ErrorCode::AlreadyJoined,
                    "this object joined already: call join only once",
                ));
            }
        }
        state.phase = Phase::Connecting;
        state.group = connection.group().to_owned();
        state.connection = Some(connection.clone());
        state.publish_change(now);
        Ok(())
    }

    /// The `join` future was dropped before it finished: back to `notJoined`, with the tag
    /// kept.
    pub(super) fn cancel_join(&self) {
        let now = self.clock.now();
        let mut state = self.lock();
        if matches!(state.phase, Phase::Connecting) {
            state.phase = Phase::NotJoined;
            state.publish_change(now);
        }
    }

    /// `join` failed on an error a retry cannot fix.
    pub(super) fn join_failed(&self, error: &Error) {
        let now = self.clock.now();
        let mut state = self.lock();
        if matches!(state.phase, Phase::Connecting) {
            state.phase = Phase::NotJoined;
            state.last_error = Some(error.to_string());
            state.publish_change(now);
        }
    }

    /// The tag for a join attempt sent now. The first attempt with a tag keeps its send time.
    pub(super) fn tag_for_attempt(&self) -> u128 {
        let now = self.clock.now();
        let mut state = self.lock();
        if state.first_attempt_sent_at.is_none() {
            state.first_attempt_sent_at = Some(now);
        }
        state.next_tag
    }

    /// The send time of the first join attempt with the current tag.
    pub(super) fn first_attempt_sent_at(&self) -> Duration {
        let now = self.clock.now();
        self.lock().first_attempt_sent_at.unwrap_or(now)
    }

    /// The next join is a new membership: a new tag (election-core, "Membership tag").
    pub(super) fn new_tag(&self) {
        self.lock().forget_tag();
    }

    /// Keep a place from a join or a renew. `false` when `leave` came first: the caller
    /// stops.
    pub(super) fn hold(&self, place: Place) -> bool {
        let now = self.clock.now();
        let mut state = self.lock();
        if matches!(state.phase, Phase::Left) {
            return false;
        }
        state.phase = Phase::Holding(place);
        state.last_error = None;
        state.publish_change(now);
        true
    }

    /// The database says my place is gone: join again with a new membership.
    pub(super) fn place_lost(&self) {
        let now = self.clock.now();
        let mut state = self.lock();
        if matches!(state.phase, Phase::Holding(_)) {
            state.phase = Phase::Reconnecting;
            state.forget_tag();
            state.publish_change(now);
        }
    }

    /// A database call failed: keep its message for the status and log it at debug.
    pub(super) fn call_failed(&self, call: &str, message: &str) {
        let now = self.clock.now();
        let mut state = self.lock();
        log_failed_call(&state.group, call, message);
        state.last_error = Some(message.to_owned());
        // The lease may have ended during the call.
        state.publish_change(now);
    }

    /// Send the status to the subscribers if it changed (after a lease deadline).
    pub(super) fn publish_status(&self) {
        let now = self.clock.now();
        self.lock().publish_change(now);
    }

    /// Start `leave`: the status is `left` at once, the last notification is sent, the
    /// streams end and the pending join and the driver stop. `None` when there is nothing
    /// to remove from the database (no place, or `leave` ran already).
    pub(super) fn start_leave(&self) -> Option<PlaceToRemove> {
        let now = self.clock.now();
        let mut state = self.lock();
        if matches!(state.phase, Phase::Left) {
            return None;
        }
        let membership = match &state.phase {
            Phase::Holding(place) => Some(place.membership),
            _ => None,
        };
        state.phase = Phase::Left;
        state.publish_change(now);
        // Dropping the queues ends the streams after `left` (rust-api, "End of the stream").
        state.subscribers.clear();
        state.warning_subscribers.clear();
        let connection = state.connection.clone();
        drop(state);
        // `send` stores nothing when no receiver exists; `send_replace` always stores.
        self.leaving.send_replace(true);

        match (connection, membership) {
            (Some(connection), Some(membership)) => Some(PlaceToRemove {
                connection,
                membership,
            }),
            _ => None,
        }
    }
}

impl State {
    fn status(&self, now: Duration) -> Status {
        let reason = match &self.phase {
            Phase::NotJoined => OfflineReason::NotJoined,
            Phase::Connecting => OfflineReason::Connecting,
            Phase::Reconnecting => OfflineReason::Reconnecting,
            Phase::Left => OfflineReason::Left,
            Phase::Holding(place) => {
                if now >= place.lease_end {
                    OfflineReason::ConnectionLost
                } else if place.is_first_in_line() {
                    return Status::Master;
                } else {
                    return Status::Slave;
                }
            }
        };
        Status::Offline {
            reason,
            error: self.last_error.clone(),
        }
    }

    fn token(&self, now: Duration) -> Option<Token> {
        let Phase::Holding(place) = &self.phase else {
            return None;
        };
        if self.status(now) != Status::Master {
            return None;
        }
        Some(Token {
            generation: place.view.generation,
            number: place.membership.number,
        })
    }

    fn forget_tag(&mut self) {
        self.next_tag = rand::random();
        self.first_attempt_sent_at = None;
    }

    /// Send the status to every subscriber and log it, when its state or reason changed, and
    /// give it. A new error message alone is not a change (status-reporting).
    fn publish_change(&mut self, now: Duration) -> Status {
        let status = self.status(now);
        if same_state_and_reason(&status, &self.last_published) {
            return status;
        }
        self.log_status_change(&status);
        self.subscribers
            .retain(|subscriber| subscriber.send(status.clone()).is_ok());
        self.last_published = status.clone();
        status
    }

    fn log_status_change(&self, status: &Status) {
        let (number, generation) = match &self.phase {
            Phase::Holding(place) => (Some(place.membership.number), Some(place.view.generation)),
            _ => (None, None),
        };
        let reason = match status {
            Status::Offline { reason, .. } => Some(reason.to_string()),
            Status::Master | Status::Slave => None,
        };
        tracing::info!(
            target: "doyen",
            group = self.group.as_str(),
            state = state_name(status),
            reason,
            number,
            generation,
            "the status changed"
        );
    }
}

fn same_state_and_reason(first: &Status, second: &Status) -> bool {
    match (first, second) {
        (Status::Master, Status::Master) | (Status::Slave, Status::Slave) => true,
        (
            Status::Offline {
                reason: first_reason,
                ..
            },
            Status::Offline {
                reason: second_reason,
                ..
            },
        ) => first_reason == second_reason,
        _ => false,
    }
}

/// The state as the same text in every language (status-reporting, "States and reasons are
/// strings").
fn state_name(status: &Status) -> &'static str {
    match status {
        Status::Master => "master",
        Status::Slave => "slave",
        Status::Offline { .. } => "offline",
    }
}

/// A failed database call, at debug. The adapters never put a secret in a message.
pub(super) fn log_failed_call(group: &str, call: &str, message: &str) {
    tracing::debug!(
        target: "doyen",
        group,
        call,
        error = message,
        "a database call failed"
    );
}

pub(super) fn left_error() -> Error {
    Error::new(
        ErrorCode::Left,
        "this object called leave: create a new object to join again",
    )
}
