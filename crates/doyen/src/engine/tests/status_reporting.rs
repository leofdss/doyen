// SPDX-License-Identifier: MIT OR Apache-2.0
//! The scenarios of the spec status-reporting ("Log a reason" is in the status module).

use std::time::Duration;

use futures_util::StreamExt;
use tokio::time::sleep;
use tracing::Level;

use super::helpers::*;
use crate::backend::Warning;
use crate::conformance::TestDatabase;
use crate::error::{Error, ErrorCode};
use crate::memory::{Fault, Operation};
use crate::status::{OfflineReason, Status};

fn reasons(notifications: &[Status]) -> Vec<Option<OfflineReason>> {
    notifications.iter().map(reason_of).collect()
}

/// Read the notifications until the last one is `last`.
async fn notifications_until(
    changes: &mut (impl futures_core::Stream<Item = Status> + Unpin),
    last: Status,
) -> Vec<Status> {
    let mut notifications = Vec::new();
    wait_until(Duration::from_secs(30), || {
        notifications.extend(ready_notifications(changes));
        notifications.last() == Some(&last)
    })
    .await;
    notifications
}

// spec: status-reporting / New object
#[tokio::test(start_paused = true)]
async fn a_new_object_is_offline_not_joined() {
    let group = TestGroup::new();

    let member = group.member();

    assert_eq!(
        member.status(),
        Status::Offline {
            reason: OfflineReason::NotJoined,
            error: None
        }
    );
}

// spec: status-reporting / Joined
#[tokio::test(start_paused = true)]
async fn a_joined_object_is_master_or_slave() {
    let group = TestGroup::new();

    let first = group.joined_member().await;
    let second = group.joined_member().await;

    assert_eq!(first.status(), Status::Master);
    assert_eq!(second.status(), Status::Slave);
}

// spec: status-reporting / Join running
#[tokio::test(start_paused = true)]
async fn a_running_join_is_offline_connecting() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_fault_on_every_call(Fault::NoAnswer);
    let doyen = member.doyen.clone();
    let config = group.config();
    let _join = tokio::spawn(async move { doyen.join(config).await });

    sleep(Duration::from_secs(5)).await;

    assert!(is_offline_with(&member.status(), OfflineReason::Connecting));
}

// spec: status-reporting / Join failed
#[tokio::test(start_paused = true)]
async fn a_failed_join_is_offline_not_joined() {
    let group = TestGroup::new();
    let member = group.member();
    let refused = Error::new(ErrorCode::Auth, "the database refused the login");
    member
        .adapter
        .set_fault(Operation::Connect, Fault::Refused(refused));

    let result = member.doyen.join(group.config()).await;

    assert_eq!(result.expect_err("join fails").code(), ErrorCode::Auth);
    assert!(is_offline_with(&member.status(), OfflineReason::NotJoined));
}

// spec: status-reporting / Short outage longer than the lease
#[tokio::test(start_paused = true)]
async fn an_outage_shorter_than_the_timeout_keeps_the_place() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let number = member.number();
    member.wait_for_next_renew().await;
    let mut changes = member.doyen.status_changes();

    member.adapter.set_fault_on_every_call(Fault::NoAnswer);
    sleep(Duration::from_millis(9_500)).await;
    member.adapter.clear_faults();
    let notifications = notifications_until(&mut changes, Status::Master).await;

    assert_eq!(
        reasons(&notifications),
        vec![Some(OfflineReason::ConnectionLost), None]
    );
    assert_eq!(member.number(), number);
}

// spec: status-reporting / Outage longer than the timeout
#[tokio::test(start_paused = true)]
async fn an_outage_longer_than_the_timeout_gives_a_new_number() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let old_number = member.number().expect("a place");
    member.wait_for_next_renew().await;
    let mut changes = member.doyen.status_changes();

    member.adapter.set_fault_on_every_call(Fault::NoAnswer);
    sleep(Duration::from_secs(12)).await;
    member.adapter.clear_faults();
    let notifications = notifications_until(&mut changes, Status::Master).await;

    assert_eq!(
        reasons(&notifications),
        vec![
            Some(OfflineReason::ConnectionLost),
            Some(OfflineReason::Reconnecting),
            None
        ]
    );
    let new_number = member.number().expect("a new place");
    assert!(new_number > old_number);
}

// spec: status-reporting / Network error while reconnecting
#[tokio::test(start_paused = true)]
async fn a_network_error_while_reconnecting_keeps_reconnecting() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let mut changes = member.doyen.status_changes();
    member.adapter.set_fault(
        Operation::Join,
        Fault::NetworkError("connection refused".to_owned()),
    );

    group.database.lose_all_data(&group.name).await;
    wait_until(Duration::from_secs(5), || {
        is_offline_with(&member.status(), OfflineReason::Reconnecting)
    })
    .await;
    let reconnecting_all_the_time = always_during(Duration::from_secs(5), || {
        is_offline_with(&member.status(), OfflineReason::Reconnecting)
    })
    .await;

    assert!(reconnecting_all_the_time);
    let error = error_of(&member.status()).expect("the join error");
    assert!(error.contains("connection refused"), "{error}");
    assert_eq!(
        reasons(&ready_notifications(&mut changes)),
        vec![Some(OfflineReason::Reconnecting)]
    );
}

// spec: status-reporting / After leave
#[tokio::test(start_paused = true)]
async fn after_leave_the_status_stays_left() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let mut changes = member.doyen.status_changes();

    member.doyen.leave().await.expect("leave works");
    sleep(Duration::from_secs(20)).await;

    assert!(is_offline_with(&member.status(), OfflineReason::Left));
    assert_eq!(
        reasons(&ready_notifications(&mut changes)),
        vec![Some(OfflineReason::Left)]
    );
    assert_eq!(changes.next().await, None);
}

// spec: status-reporting / Wrong password after join
#[tokio::test(start_paused = true)]
async fn a_refused_login_after_join_shows_in_the_status() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let refused = Error::new(
        ErrorCode::Auth,
        "the database refused the login: WRONGPASS invalid username-password pair",
    );

    member
        .adapter
        .set_fault_on_every_call(Fault::Refused(refused));
    wait_until(Duration::from_secs(15), || {
        member.status() != Status::Master
    })
    .await;

    let status = member.status();
    assert!(is_offline_with(&status, OfflineReason::ConnectionLost));
    let error = error_of(&status).expect("the login error");
    assert!(error.contains("WRONGPASS"), "{error}");
}

// spec: status-reporting / Master change
#[tokio::test(start_paused = true)]
async fn each_subscriber_gets_one_notification_when_a_slave_becomes_master() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    let mut first_stream = second.doyen.status_changes();
    let mut second_stream = second.doyen.status_changes();

    first.doyen.leave().await.expect("leave works");
    wait_until(Duration::from_secs(5), || second.status() == Status::Master).await;

    assert_eq!(ready_notifications(&mut first_stream), vec![Status::Master]);
    assert_eq!(
        ready_notifications(&mut second_stream),
        vec![Status::Master]
    );
}

// spec: status-reporting / Error changes alone
#[tokio::test(start_paused = true)]
async fn a_new_error_message_alone_is_not_a_change() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let mut changes = member.doyen.status_changes();
    member
        .adapter
        .set_fault_on_every_call(Fault::NetworkError("first error".to_owned()));
    wait_until(Duration::from_secs(15), || {
        is_offline_with(&member.status(), OfflineReason::ConnectionLost)
    })
    .await;
    assert_eq!(ready_notifications(&mut changes).len(), 1);

    member
        .adapter
        .set_fault_on_every_call(Fault::NetworkError("second error".to_owned()));
    member.wait_for_next_renew().await;
    sleep(STEP).await;

    assert_eq!(ready_notifications(&mut changes), Vec::<Status>::new());
    assert_eq!(error_of(&member.status()), Some("second error".to_owned()));
}

// spec: status-reporting / Slow subscriber
#[tokio::test(start_paused = true)]
async fn a_slow_subscriber_gets_every_change_in_order() {
    let group = TestGroup::new();
    let member = group.member();
    let mut changes = member.doyen.status_changes();

    member.doyen.join(group.config()).await.expect("join works");
    member.doyen.leave().await.expect("leave works");
    sleep(Duration::from_secs(5)).await;

    let notifications: Vec<Status> = changes.by_ref().collect().await;
    assert_eq!(
        reasons(&notifications),
        vec![
            Some(OfflineReason::Connecting),
            None,
            Some(OfflineReason::Left)
        ]
    );
    assert_eq!(notifications[1], Status::Master);
}

// spec: status-reporting / Subscribe, then join
#[tokio::test(start_paused = true)]
async fn a_subscriber_before_join_gets_connecting_then_master() {
    let group = TestGroup::new();
    let member = group.member();
    let mut changes = member.doyen.status_changes();

    member.doyen.join(group.config()).await.expect("join works");

    let notifications = ready_notifications(&mut changes);
    assert_eq!(notifications.len(), 2, "{notifications:?}");
    assert!(is_offline_with(
        &notifications[0],
        OfflineReason::Connecting
    ));
    assert_eq!(notifications[1], Status::Master);
}

// spec: status-reporting / Leave
#[tokio::test(start_paused = true)]
async fn the_last_notification_is_left() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let mut changes = member.doyen.status_changes();

    member.doyen.leave().await.expect("leave works");

    let notifications: Vec<Status> = changes.by_ref().collect().await;
    let last = notifications.last().expect("one notification");
    assert!(is_offline_with(last, OfflineReason::Left));
}

// spec: status-reporting / Master change in the log
#[tokio::test(start_paused = true)]
async fn a_new_master_writes_one_info_event() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    let (log, _guard) = EventLog::start();

    first.doyen.leave().await.expect("leave works");
    wait_until(Duration::from_secs(5), || second.status() == Status::Master).await;

    let master_events: Vec<LoggedEvent> = log
        .doyen_events(Level::INFO)
        .into_iter()
        .filter(|event| event.field("state") == Some("master"))
        .collect();
    assert_eq!(master_events.len(), 1, "{master_events:?}");
    let event = &master_events[0];
    let token = second.doyen.token().expect("master");
    assert_eq!(event.field("group"), Some("billing"));
    assert_eq!(
        event.field("number"),
        Some(token.number.to_string().as_str())
    );
    assert_eq!(
        event.field("generation"),
        Some(token.generation.to_string().as_str())
    );
}

// spec: status-reporting / Failed call in the log
#[tokio::test(start_paused = true)]
async fn a_failed_renew_writes_one_debug_event_and_no_info_event() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    member.wait_for_next_renew().await;
    let (log, _guard) = EventLog::start();

    member.adapter.set_fault(
        Operation::Renew,
        Fault::NetworkError("connection reset by peer".to_owned()),
    );
    member.wait_for_next_renew().await;
    member.adapter.clear_faults();
    sleep(STEP).await;

    let debug_events = log.doyen_events(Level::DEBUG);
    assert_eq!(debug_events.len(), 1, "{debug_events:?}");
    let has_message = debug_events[0]
        .fields
        .values()
        .any(|value| value.contains("connection reset by peer"));
    assert!(has_message, "{debug_events:?}");
    assert_eq!(log.doyen_events(Level::INFO).len(), 0);
    assert_eq!(member.status(), Status::Master);
}

// spec: status-reporting / Setup with a known risk
#[tokio::test(start_paused = true)]
async fn a_connect_warning_is_logged_once_and_changes_no_status() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_connect_warnings(vec![Warning {
        code: "DOYEN_MEMORY_RISK".to_owned(),
        message: "the in-memory server may lose data".to_owned(),
    }]);
    let mut changes = member.doyen.status_changes();
    let (log, _guard) = EventLog::start();

    member.doyen.join(group.config()).await.expect("join works");

    let warnings = log.doyen_events(Level::WARN);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].field("code"), Some("DOYEN_MEMORY_RISK"));
    sleep(Duration::from_secs(10)).await;
    assert_eq!(log.doyen_events(Level::WARN).len(), 1);
    assert_eq!(member.status(), Status::Master);
    assert_eq!(
        reasons(&ready_notifications(&mut changes)),
        vec![Some(OfflineReason::Connecting), None]
    );
}
