// SPDX-License-Identifier: MIT OR Apache-2.0
//! Helpers of the engine tests: members of a group on one in-memory database, a host clock
//! that can be suspended, waits in test time, the notifications and the log events.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use futures_core::Stream;
use futures_util::{FutureExt, StreamExt};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

use crate::backend::Backend;
use crate::clock::{Clock, FakeClock};
use crate::config::{DatabaseConfig, RedisConfig};
use crate::engine::Doyen;
use crate::memory::{MemoryBackend, MemoryDatabase, Operation};
use crate::status::{OfflineReason, Status};

/// The step of the waits: the tests see the status every 10 ms of test time.
pub(super) const STEP: Duration = Duration::from_millis(10);

/// The defaults of the config, as the specs give them.
pub(super) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const DEFAULT_RENEW_INTERVAL: Duration = Duration::from_secs(2);

/// The clock of the app host: the fake clock, plus the time of each suspend. A suspend moves
/// this clock but not the Tokio timers, like a real suspend.
#[derive(Clone)]
pub(super) struct HostClock {
    fake: FakeClock,
    suspended: Arc<Mutex<Duration>>,
}

impl HostClock {
    pub(super) fn new() -> HostClock {
        HostClock {
            fake: FakeClock::new(),
            suspended: Arc::new(Mutex::new(Duration::ZERO)),
        }
    }

    /// The host sleeps for `duration` and wakes up. No task runs meanwhile.
    pub(super) fn suspend(&self, duration: Duration) {
        *self.suspended.lock().expect("the clock lock") += duration;
    }
}

impl Clock for HostClock {
    fn now(&self) -> Duration {
        self.fake.now() + *self.suspended.lock().expect("the clock lock")
    }
}

/// The members of one group on one in-memory database. The test must run in paused Tokio
/// time (`#[tokio::test(start_paused = true)]`).
pub(super) struct TestGroup {
    pub(super) database: MemoryDatabase,
    pub(super) clock: HostClock,
    pub(super) name: String,
}

impl TestGroup {
    pub(super) fn new() -> TestGroup {
        TestGroup::named("billing")
    }

    pub(super) fn named(name: &str) -> TestGroup {
        TestGroup {
            database: MemoryDatabase::new(),
            clock: HostClock::new(),
            name: name.to_owned(),
        }
    }

    /// Another group on the same database.
    pub(super) fn other_group(&self, name: &str) -> TestGroup {
        TestGroup {
            database: self.database.clone(),
            clock: self.clock.clone(),
            name: name.to_owned(),
        }
    }

    /// The config with only the required settings.
    pub(super) fn config(&self) -> DatabaseConfig {
        DatabaseConfig::Redis(RedisConfig::new("redis://localhost", self.name.clone()))
    }

    pub(super) fn config_with_timing(
        &self,
        timeout_ms: u64,
        renew_interval_ms: u64,
    ) -> DatabaseConfig {
        let mut config = RedisConfig::new("redis://localhost", self.name.clone());
        config.timeout_ms = Some(timeout_ms);
        config.renew_interval_ms = Some(renew_interval_ms);
        DatabaseConfig::Redis(config)
    }

    /// A new app instance that did not join.
    pub(super) fn member(&self) -> Member {
        let adapter = Arc::new(self.database.backend());
        let backend: Arc<dyn Backend> = adapter.clone();
        let clock: Arc<dyn Clock> = Arc::new(self.clock.clone());
        Member {
            doyen: Doyen::with_backend(backend, clock),
            adapter,
        }
    }

    /// A new app instance that joined with the defaults.
    pub(super) async fn joined_member(&self) -> Member {
        let member = self.member();
        member
            .doyen
            .join(self.config())
            .await
            .expect("join must work");
        member
    }

    /// The live numbers in the database now.
    pub(super) fn live_numbers(&self) -> Vec<u64> {
        self.database.live_numbers(&self.name)
    }
}

/// One app instance: the object and its adapter (its network to the database).
pub(super) struct Member {
    pub(super) doyen: Doyen,
    pub(super) adapter: Arc<MemoryBackend>,
}

impl Member {
    pub(super) fn number(&self) -> Option<u64> {
        self.doyen.number()
    }

    pub(super) fn status(&self) -> Status {
        self.doyen.status()
    }

    pub(super) fn renew_count(&self) -> usize {
        self.adapter.call_count(Operation::Renew)
    }

    /// Wait until this member sent one more renew.
    pub(super) async fn wait_for_next_renew(&self) {
        let count = self.renew_count();
        wait_until(Duration::from_secs(30), || self.renew_count() > count).await;
    }
}

/// The reason of an offline status, `None` for master and slave.
pub(super) fn reason_of(status: &Status) -> Option<OfflineReason> {
    match status {
        Status::Offline { reason, .. } => Some(*reason),
        Status::Master | Status::Slave => None,
    }
}

/// The error message of an offline status.
pub(super) fn error_of(status: &Status) -> Option<String> {
    match status {
        Status::Offline { error, .. } => error.clone(),
        Status::Master | Status::Slave => None,
    }
}

pub(super) fn is_offline_with(status: &Status, expected: OfflineReason) -> bool {
    reason_of(status) == Some(expected)
}

/// Wait in steps of [`STEP`] of test time until `condition` holds, and give the time it
/// took. Fails the test after `limit`.
pub(super) async fn wait_until(limit: Duration, mut condition: impl FnMut() -> bool) -> Duration {
    let start = tokio::time::Instant::now();
    while !condition() {
        let waited = start.elapsed();
        assert!(
            waited <= limit,
            "the condition did not hold within {limit:?}"
        );
        tokio::time::sleep(STEP).await;
    }
    start.elapsed()
}

/// Check `condition` every [`STEP`] for `duration` of test time; `true` when it always held.
pub(super) async fn always_during(duration: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let start = tokio::time::Instant::now();
    while start.elapsed() < duration {
        if !condition() {
            return false;
        }
        tokio::time::sleep(STEP).await;
    }
    condition()
}

/// The notifications that a stream holds now, without waiting.
pub(super) fn ready_notifications(
    stream: &mut (impl Stream<Item = Status> + Unpin),
) -> Vec<Status> {
    let mut notifications = Vec::new();
    while let Some(Some(status)) = stream.next().now_or_never() {
        notifications.push(status);
    }
    notifications
}

/// One `tracing` event, with its fields as text.
#[derive(Debug, Clone)]
pub(super) struct LoggedEvent {
    pub(super) level: Level,
    pub(super) target: String,
    pub(super) fields: HashMap<String, String>,
}

impl LoggedEvent {
    pub(super) fn field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(String::as_str)
    }
}

/// The events of one test, for the log tests. `start` makes it the log of the current
/// thread, where the tasks of a `#[tokio::test]` run.
///
/// One global subscriber sends each event to the log of its thread. A subscriber per thread
/// (`tracing::subscriber::set_default`) is not safe here: when another test thread meets a
/// log call for the first time at the same moment, `tracing` can keep "nobody listens" for
/// that call, and the event is lost.
#[derive(Clone, Default)]
pub(super) struct EventLog {
    events: Arc<Mutex<Vec<LoggedEvent>>>,
}

thread_local! {
    static LOG_OF_THIS_THREAD: RefCell<Option<EventLog>> = const { RefCell::new(None) };
}

/// Ends the log of the thread when the test drops it.
pub(super) struct LogGuard;

impl Drop for LogGuard {
    fn drop(&mut self) {
        LOG_OF_THIS_THREAD.with(|log| *log.borrow_mut() = None);
    }
}

impl EventLog {
    pub(super) fn start() -> (EventLog, LogGuard) {
        static GLOBAL_SUBSCRIBER: Once = Once::new();
        GLOBAL_SUBSCRIBER.call_once(|| {
            tracing::subscriber::set_global_default(EventsToThreadLog)
                .expect("no other global subscriber in the tests");
        });
        let log = EventLog::default();
        LOG_OF_THIS_THREAD.with(|current| *current.borrow_mut() = Some(log.clone()));
        (log, LogGuard)
    }

    pub(super) fn events(&self) -> Vec<LoggedEvent> {
        self.events.lock().expect("the event log lock").clone()
    }

    /// The events of target `doyen` at this level.
    pub(super) fn doyen_events(&self, level: Level) -> Vec<LoggedEvent> {
        self.events()
            .into_iter()
            .filter(|event| event.target == "doyen" && event.level == level)
            .collect()
    }
}

/// Collects the fields of one event as text.
struct FieldsAsText(HashMap<String, String>);

impl Visit for FieldsAsText {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }
}

/// The global subscriber of the tests: it keeps each event in the log of its thread, when
/// the thread has one.
struct EventsToThreadLog;

impl Subscriber for EventsToThreadLog {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let Some(log) = LOG_OF_THIS_THREAD.with(|log| log.borrow().clone()) else {
            return;
        };
        let mut fields = FieldsAsText(HashMap::new());
        event.record(&mut fields);
        let logged = LoggedEvent {
            level: *event.metadata().level(),
            target: event.metadata().target().to_owned(),
            fields: fields.0,
        };
        log.events.lock().expect("the event log lock").push(logged);
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
