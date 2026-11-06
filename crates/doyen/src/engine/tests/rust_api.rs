// SPDX-License-Identifier: MIT OR Apache-2.0
//! The scenarios of the spec rust-api that need the engine.

use std::future::Future;
use std::pin::pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use futures_util::StreamExt;
use tokio::time::{sleep, timeout};

use super::helpers::*;
use crate::backend::Backend;
use crate::clock::Clock;
use crate::config::{DatabaseConfig, RedisConfig};
use crate::engine::Doyen;
use crate::error::{Error, ErrorCode};
use crate::memory::{Fault, MemoryDatabase, Operation};
use crate::status::{OfflineReason, Status};
use crate::token::Token;

/// A clock that moves only when the test moves it, without Tokio: it still works after the
/// runtime is gone.
#[derive(Clone, Default)]
struct ManualClock {
    now: Arc<Mutex<Duration>>,
}

impl ManualClock {
    fn advance(&self, duration: Duration) {
        *self.now.lock().expect("the clock lock") += duration;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.now.lock().expect("the clock lock")
    }
}

/// Run a future to its end on this thread, without Tokio, like
/// `futures::executor::block_on`. A short busy loop is enough for a test.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }
}

// spec: rust-api / New object
#[tokio::test(start_paused = true)]
async fn a_new_object_is_not_joined_and_calls_no_database() {
    fn assert_send_and_sync<T: Send + Sync>() {}
    assert_send_and_sync::<Doyen>();
    let group = TestGroup::new();

    let member = group.member();
    sleep(Duration::from_secs(5)).await;

    assert!(matches!(
        member.status(),
        Status::Offline {
            reason: OfflineReason::NotJoined,
            ..
        }
    ));
    for operation in [
        Operation::Connect,
        Operation::Join,
        Operation::Renew,
        Operation::Leave,
        Operation::Watch,
    ] {
        assert_eq!(member.adapter.call_count(operation), 0, "{operation:?}");
    }
    assert!(is_offline_with(
        &Doyen::new().status(),
        OfflineReason::NotJoined
    ));
}

// spec: rust-api / Clone
#[tokio::test(start_paused = true)]
async fn a_clone_shares_the_membership() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let clone = member.doyen.clone();

    clone.leave().await.expect("leave works");

    assert!(is_offline_with(&member.status(), OfflineReason::Left));
}

/// The app code of the scenario "Typical use", with `?`.
async fn run_the_app(
    doyen: &Doyen,
    config: DatabaseConfig,
) -> Result<(Status, Option<Token>), Error> {
    doyen.join(config).await?;
    let status: Status = doyen.status();
    let token: Option<Token> = doyen.token();
    doyen.leave().await?;
    Ok((status, token))
}

// spec: rust-api / Typical use
#[tokio::test(start_paused = true)]
async fn the_typical_use_compiles_and_works() {
    let group = TestGroup::new();
    let member = group.member();

    let (status, token) = run_the_app(&member.doyen, group.config())
        .await
        .expect("the app works");

    assert_eq!(status, Status::Master);
    assert_eq!(token.map(|token| token.number), Some(1));
    assert!(is_offline_with(&member.status(), OfflineReason::Left));
}

// spec: rust-api / Two streams
#[tokio::test(start_paused = true)]
async fn two_streams_get_the_same_change() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    let mut one_stream = second.doyen.status_changes();
    let mut other_stream = second.doyen.status_changes();

    first.doyen.leave().await.expect("leave works");
    wait_until(Duration::from_secs(5), || second.status() == Status::Master).await;

    assert_eq!(one_stream.next().await, Some(Status::Master));
    assert_eq!(other_stream.next().await, Some(Status::Master));
}

// spec: rust-api / End of the stream
#[tokio::test(start_paused = true)]
async fn the_stream_ends_after_left() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let mut changes = member.doyen.status_changes();

    member.doyen.leave().await.expect("leave works");

    let last = changes.next().await.expect("the left status");
    assert!(matches!(
        last,
        Status::Offline {
            reason: OfflineReason::Left,
            ..
        }
    ));
    assert_eq!(changes.next().await, None);
    assert_eq!(member.doyen.status_changes().next().await, None);
}

// spec: rust-api / Join with a timeout
#[tokio::test(start_paused = true)]
async fn a_join_dropped_by_a_timeout_is_not_joined_and_keeps_its_tag() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_fault(Operation::Join, Fault::LostAnswer);

    let result = timeout(Duration::from_secs(1), member.doyen.join(group.config())).await;

    assert!(result.is_err(), "the timeout must end first");
    assert!(is_offline_with(&member.status(), OfflineReason::NotJoined));
    let written = group.live_numbers();
    member.adapter.clear_faults();
    member.doyen.join(group.config()).await.expect("join works");
    assert_eq!(written.len(), 1);
    assert_eq!(group.live_numbers(), written);
    assert_eq!(member.number(), Some(written[0]));
}

// spec: rust-api / Runtime shut down
#[test]
fn after_the_runtime_shuts_down_nothing_renews_and_the_lease_ends() {
    let clock = ManualClock::default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("a runtime");
    let (doyen, adapter) = runtime.block_on(async {
        let database = MemoryDatabase::new();
        let adapter = Arc::new(database.backend());
        let backend: Arc<dyn Backend> = adapter.clone();
        let doyen = Doyen::with_backend(backend, Arc::new(clock.clone()));
        let config = DatabaseConfig::Redis(RedisConfig::new("redis://localhost", "billing"));
        doyen.join(config).await.expect("join works");
        sleep(Duration::from_secs(5)).await;
        (doyen, adapter)
    });
    assert_eq!(doyen.status(), Status::Master);
    assert!(adapter.call_count(Operation::Renew) >= 2);
    let renews = adapter.call_count(Operation::Renew);

    drop(runtime);
    clock.advance(Duration::from_secs(10));

    assert_ne!(doyen.status(), Status::Master);
    assert_eq!(doyen.token(), None);
    assert_eq!(adapter.call_count(Operation::Renew), renews);
}

// spec: rust-api / No Tokio runtime
#[test]
fn join_outside_a_tokio_runtime_fails_with_doyen_unsupported() {
    let database = MemoryDatabase::new();
    let backend: Arc<dyn Backend> = Arc::new(database.backend());
    let doyen = Doyen::with_backend(backend, Arc::new(ManualClock::default()));
    let config = DatabaseConfig::Redis(RedisConfig::new("redis://localhost", "billing"));

    let result = block_on(doyen.join(config));

    let error = result.expect_err("join fails");
    assert_eq!(error.code(), ErrorCode::Unsupported);
    assert!(error.message().contains("Tokio runtime"), "{error}");
    assert!(is_offline_with(&doyen.status(), OfflineReason::NotJoined));
}

// spec: rust-api / Object dropped while master
#[tokio::test(start_paused = true)]
async fn dropping_the_last_handle_of_the_master_stops_its_renews() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    first.wait_for_next_renew().await;
    let Member {
        doyen,
        adapter: first_adapter,
    } = first;
    let renews = first_adapter.call_count(Operation::Renew);

    drop(doyen);
    let waited = wait_until(Duration::from_secs(20), || {
        second.status() == Status::Master
    })
    .await;

    assert!(
        waited >= Duration::from_secs(8) && waited <= Duration::from_secs(12),
        "master after {waited:?}"
    );
    assert_eq!(first_adapter.call_count(Operation::Renew), renews);
}

// spec: rust-api / Clone still alive
#[tokio::test(start_paused = true)]
async fn a_living_clone_keeps_the_membership() {
    let group = TestGroup::new();
    let Member { doyen, adapter } = group.joined_member().await;
    let clone = doyen.clone();
    let renews = adapter.call_count(Operation::Renew);

    drop(doyen);
    sleep(Duration::from_secs(20)).await;

    assert!(adapter.call_count(Operation::Renew) >= renews + 9);
    assert_eq!(clone.status(), Status::Master);
}
