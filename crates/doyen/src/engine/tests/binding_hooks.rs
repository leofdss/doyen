// SPDX-License-Identifier: MIT OR Apache-2.0
//! The hidden hooks for the bindings (`__binding`). No scenario of this change names them
//! (the binding specs test what an app sees), so these are plain unit tests.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures_core::Stream;
use futures_util::{FutureExt, StreamExt};
use tokio::time::sleep;

use super::helpers::*;
use crate::__binding::{self, Warning};
use crate::status::{OfflineReason, Status};

/// A liveness check that the test turns on and off, like the tick of the Node.js binding.
struct LivenessSwitch {
    live: Arc<AtomicBool>,
}

impl LivenessSwitch {
    /// Give the member a liveness check that returns `true` until the test sets `false`.
    fn install(member: &Member) -> LivenessSwitch {
        let live = Arc::new(AtomicBool::new(true));
        let read_by_the_check = live.clone();
        __binding::set_liveness(&member.doyen, move || {
            read_by_the_check.load(Ordering::SeqCst)
        });
        LivenessSwitch { live }
    }

    fn set(&self, live: bool) {
        self.live.store(live, Ordering::SeqCst);
    }
}

fn two_warnings() -> Vec<Warning> {
    vec![
        Warning {
            code: "DOYEN_MEMORY_RISK".to_owned(),
            message: "the in-memory server may lose data".to_owned(),
        },
        Warning {
            code: "DOYEN_MEMORY_SLOW".to_owned(),
            message: "the in-memory server is slow".to_owned(),
        },
    ]
}

/// The warnings that a stream holds now, without waiting.
fn ready_warnings(stream: &mut (impl Stream<Item = Warning> + Unpin)) -> Vec<Warning> {
    let mut warnings = Vec::new();
    while let Some(Some(warning)) = stream.next().now_or_never() {
        warnings.push(warning);
    }
    warnings
}

fn reasons(notifications: &[Status]) -> Vec<Option<OfflineReason>> {
    notifications.iter().map(reason_of).collect()
}

#[tokio::test(start_paused = true)]
async fn a_false_liveness_check_skips_the_renews_until_the_lease_ends() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let liveness = LivenessSwitch::install(&member);
    member.wait_for_next_renew().await;
    let renews_before = member.renew_count();
    let mut changes = member.doyen.status_changes();

    liveness.set(false);
    let waited = wait_until(Duration::from_secs(15), || {
        is_offline_with(&member.status(), OfflineReason::ConnectionLost)
    })
    .await;

    // The local lease of the last renew is 9 s (timeout − 10%).
    assert!(waited >= Duration::from_millis(8_900), "{waited:?}");
    assert!(waited <= Duration::from_millis(9_100), "{waited:?}");
    assert_eq!(member.renew_count(), renews_before);
    assert_eq!(
        reasons(&ready_notifications(&mut changes)),
        vec![Some(OfflineReason::ConnectionLost)]
    );
}

#[tokio::test(start_paused = true)]
async fn a_true_liveness_check_again_gets_the_same_place_back() {
    let group = TestGroup::new();
    let member = group.member();
    // Like a binding: the check is set before `join`.
    let liveness = LivenessSwitch::install(&member);
    // The lease ends 3 s before the place expires in the database, and a renew comes each
    // second, so a renew reaches the place while it is still alive.
    member
        .doyen
        .join(group.config_with_timing(30_000, 1_000))
        .await
        .expect("join works");
    let number = member.number();
    member.wait_for_next_renew().await;
    liveness.set(false);
    wait_until(Duration::from_secs(30), || {
        is_offline_with(&member.status(), OfflineReason::ConnectionLost)
    })
    .await;
    let mut changes = member.doyen.status_changes();

    liveness.set(true);
    wait_until(Duration::from_secs(2), || member.status() == Status::Master).await;

    assert_eq!(member.number(), number);
    assert_eq!(reasons(&ready_notifications(&mut changes)), vec![None]);
}

#[tokio::test(start_paused = true)]
async fn the_warnings_stream_gets_each_connect_warning_once() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_connect_warnings(two_warnings());
    let mut warnings = __binding::warnings(&member.doyen);

    member.doyen.join(group.config()).await.expect("join works");
    let before_join_finished = ready_warnings(&mut warnings);
    sleep(Duration::from_secs(10)).await;
    let later = ready_warnings(&mut warnings);

    assert_eq!(before_join_finished, two_warnings());
    assert_eq!(later, Vec::new());
    assert_eq!(member.status(), Status::Master);
}

#[tokio::test(start_paused = true)]
async fn a_second_warnings_stream_does_not_get_the_earlier_warnings() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_connect_warnings(two_warnings());
    let mut first = __binding::warnings(&member.doyen);

    member.doyen.join(group.config()).await.expect("join works");
    let mut second = __binding::warnings(&member.doyen);

    assert_eq!(ready_warnings(&mut first), two_warnings());
    assert_eq!(ready_warnings(&mut second), Vec::new());
}

#[tokio::test(start_paused = true)]
async fn the_warnings_stream_ends_after_leave() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_connect_warnings(two_warnings());
    let warnings = __binding::warnings(&member.doyen);

    member.doyen.join(group.config()).await.expect("join works");
    member.doyen.leave().await.expect("leave works");
    let every_warning = tokio::time::timeout(Duration::from_secs(5), warnings.collect::<Vec<_>>())
        .await
        .expect("the stream ends");

    assert_eq!(every_warning, two_warnings());
}
