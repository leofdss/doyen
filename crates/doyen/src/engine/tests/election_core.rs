// SPDX-License-Identifier: MIT OR Apache-2.0
//! The scenarios of the spec election-core.

use std::time::Duration;

use tokio::time::{Instant, sleep, sleep_until, timeout};

use super::helpers::*;
use crate::conformance::TestDatabase;
use crate::error::{Error, ErrorCode};
use crate::memory::{Fault, Operation};
use crate::status::{OfflineReason, Status};
use crate::token::Token;

/// The local lease with the defaults: 10 s − 10%.
const DEFAULT_LEASE: Duration = Duration::from_secs(9);

fn login_error() -> Error {
    Error::new(
        ErrorCode::Auth,
        "the database refused the login: WRONGPASS invalid username-password pair",
    )
}

/// The send time of the last renew of a member.
fn last_renew(member: &Member) -> Instant {
    let times = member.adapter.call_times(Operation::Renew);
    *times.last().expect("the member renewed at least once")
}

// spec: election-core / Three members
#[tokio::test(start_paused = true)]
async fn the_member_with_the_lowest_number_is_master() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    let third = group.joined_member().await;

    first.wait_for_next_renew().await;

    assert_eq!(
        (first.number(), second.number(), third.number()),
        (Some(1), Some(2), Some(3))
    );
    assert_eq!(first.status(), Status::Master);
    assert_eq!(second.status(), Status::Slave);
    assert_eq!(third.status(), Status::Slave);
}

// spec: election-core / Master dies
#[tokio::test(start_paused = true)]
async fn the_next_member_is_master_at_its_first_renew_after_the_expiry() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    first.wait_for_next_renew().await;
    let expiry = last_renew(&first) + DEFAULT_TIMEOUT;

    first.adapter.set_fault_on_every_call(Fault::NoAnswer);
    wait_until(Duration::from_secs(20), || {
        second.status() == Status::Master
    })
    .await;

    let now = Instant::now();
    assert!(now >= expiry, "master before the expiry");
    assert!(
        now <= expiry + DEFAULT_RENEW_INTERVAL + STEP,
        "master too late"
    );
}

// spec: election-core / Old master comes back
#[tokio::test(start_paused = true)]
async fn an_old_master_gets_a_new_number_at_the_end_of_the_line() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    let _third = group.joined_member().await;
    first.adapter.set_fault_on_every_call(Fault::NoAnswer);
    wait_until(Duration::from_secs(20), || {
        second.status() == Status::Master
    })
    .await;

    first.adapter.clear_faults();
    wait_until(Duration::from_secs(10), || first.status() == Status::Slave).await;

    let number = first.number().expect("a place again");
    assert!(number > 3, "number {number}");
    assert_eq!(second.status(), Status::Master);
    assert_eq!(group.live_numbers(), vec![2, 3, number]);
}

// spec: election-core / Join while a master exists
#[tokio::test(start_paused = true)]
async fn a_new_member_joins_as_slave() {
    let group = TestGroup::new();
    let first = group.joined_member().await;

    let second = group.joined_member().await;

    assert!(second.number() > first.number());
    assert_eq!(second.status(), Status::Slave);
    assert_eq!(first.status(), Status::Master);
}

// spec: election-core / First member
#[tokio::test(start_paused = true)]
async fn the_first_member_is_master_when_join_finishes() {
    let group = TestGroup::new();

    let member = group.joined_member().await;

    assert_eq!(member.status(), Status::Master);
}

// spec: election-core / Database down at join
#[tokio::test(start_paused = true)]
async fn join_waits_for_a_database_that_starts_later() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_fault_on_every_call(Fault::NoAnswer);
    let doyen = member.doyen.clone();
    let config = group.config();
    let join = tokio::spawn(async move { doyen.join(config).await });

    wait_until(Duration::from_secs(1), || {
        is_offline_with(&member.status(), OfflineReason::Connecting)
    })
    .await;
    let connecting_all_the_time = always_during(Duration::from_secs(30), || {
        is_offline_with(&member.status(), OfflineReason::Connecting)
    })
    .await;
    assert!(connecting_all_the_time);
    assert!(!join.is_finished());

    member.adapter.clear_faults();
    let result = timeout(Duration::from_secs(5), join).await;

    let result = result.expect("join finishes").expect("no panic");
    assert_eq!(result, Ok(()));
    assert_eq!(member.status(), Status::Master);
}

// spec: election-core / Wrong password at join
#[tokio::test(start_paused = true)]
async fn a_wrong_password_ends_join() {
    let group = TestGroup::new();
    let member = group.member();
    member
        .adapter
        .set_fault(Operation::Connect, Fault::Refused(login_error()));

    let result = member.doyen.join(group.config()).await;

    assert_eq!(result.expect_err("join fails").code(), ErrorCode::Auth);
    assert!(is_offline_with(&member.status(), OfflineReason::NotJoined));
}

// spec: election-core / Password changed while running
#[tokio::test(start_paused = true)]
async fn after_join_a_refused_login_is_retried_and_shown() {
    let group = TestGroup::new();
    let member = group.joined_member().await;

    member
        .adapter
        .set_fault_on_every_call(Fault::Refused(login_error()));
    let waited = wait_until(Duration::from_secs(15), || {
        is_offline_with(&member.status(), OfflineReason::ConnectionLost)
    })
    .await;
    let renews = member.renew_count();
    sleep(Duration::from_secs(10)).await;

    assert!(waited <= DEFAULT_LEASE, "offline after {waited:?}");
    assert!(member.renew_count() > renews, "the core gave up");
    let status = member.status();
    assert!(reason_of(&status).is_some(), "{status:?}");
    let error = error_of(&status).expect("the status shows the error");
    assert!(error.contains("WRONGPASS"), "{error}");
}

// spec: election-core / Password fixed
#[tokio::test(start_paused = true)]
async fn after_the_password_is_fixed_the_member_gets_a_place_again() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    member
        .adapter
        .set_fault_on_every_call(Fault::Refused(login_error()));
    sleep(Duration::from_secs(15)).await;
    assert!(reason_of(&member.status()).is_some());

    member.adapter.clear_faults();
    wait_until(Duration::from_secs(10), || {
        member.status() == Status::Master
    })
    .await;

    assert_eq!(group.live_numbers().len(), 1);
}

// spec: election-core / Database stops answering
#[tokio::test(start_paused = true)]
async fn the_status_turns_offline_7_to_9_seconds_after_the_database_stopped() {
    for offset_ms in [0, 300, 1_000, 1_700, 1_980] {
        let group = TestGroup::new();
        let member = group.joined_member().await;
        member.wait_for_next_renew().await;
        sleep(Duration::from_millis(offset_ms)).await;

        member.adapter.set_fault_on_every_call(Fault::NoAnswer);
        let waited = wait_until(Duration::from_secs(20), || {
            member.status() != Status::Master
        })
        .await;

        assert!(
            waited >= Duration::from_secs(7) && waited <= Duration::from_secs(9),
            "offset {offset_ms} ms: offline after {waited:?}"
        );
    }
}

// spec: election-core / Host suspended
#[tokio::test(start_paused = true)]
async fn after_a_suspend_the_member_is_not_master_before_a_new_renew() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let renews_before = member.renew_count();

    group.clock.suspend(Duration::from_secs(20));

    // No task ran since the resume: the lease alone says "not master".
    assert_ne!(member.status(), Status::Master);
    assert_eq!(member.doyen.token(), None);
    wait_until(Duration::from_secs(5), || member.status() == Status::Master).await;
    assert!(member.renew_count() > renews_before);
}

// spec: election-core / Join retried after a timeout
#[tokio::test(start_paused = true)]
async fn the_lease_of_a_retried_join_counts_from_the_first_attempt() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_fault(Operation::Join, Fault::LostAnswer);
    let doyen = member.doyen.clone();
    let config = group.config();
    let join = tokio::spawn(async move { doyen.join(config).await });
    wait_until(Duration::from_secs(1), || {
        member.adapter.call_count(Operation::Join) == 1
    })
    .await;
    let first_attempt = member.adapter.call_times(Operation::Join)[0];

    // The first attempt wrote the place, and its answer is lost: it times out after one
    // renew interval. The retry comes after a backoff of at least 80 ms.
    sleep_until(first_attempt + DEFAULT_RENEW_INTERVAL + Duration::from_millis(50)).await;
    member.adapter.clear_faults();
    join.await.expect("no panic").expect("join works");
    member.adapter.set_fault_on_every_call(Fault::NoAnswer);

    assert_eq!(member.adapter.call_count(Operation::Join), 2);
    assert_eq!(group.live_numbers().len(), 1);
    let lease_end = first_attempt + DEFAULT_LEASE;
    sleep_until(lease_end - STEP).await;
    assert_eq!(member.status(), Status::Master);
    sleep_until(lease_end).await;
    assert_ne!(member.status(), Status::Master);
}

// spec: election-core / Slow answers at the smallest timeout
#[tokio::test(start_paused = true)]
async fn slow_answers_at_the_smallest_timeout_keep_the_lease() {
    let group = TestGroup::new();
    let member = group.member();
    member
        .doyen
        .join(group.config_with_timing(6_000, 2_000))
        .await
        .expect("join works");
    let just_before_the_call_timeout = Duration::from_millis(1_990);
    member.adapter.set_fault(
        Operation::Renew,
        Fault::SlowAnswer(just_before_the_call_timeout),
    );
    let renews_before = member.renew_count();

    let master_all_the_time = always_during(Duration::from_secs(30), || {
        member.status() == Status::Master
    })
    .await;

    assert!(master_all_the_time);
    assert!(member.renew_count() >= renews_before + 14);
}

// spec: election-core / Wake-up between renews
#[tokio::test(start_paused = true)]
async fn a_wake_up_renews_at_once_and_keeps_the_schedule() {
    let group = TestGroup::new();
    let _first = group.joined_member().await;
    let second = group.joined_member().await;
    second.wait_for_next_renew().await;
    let scheduled = last_renew(&second);

    sleep_until(scheduled + Duration::from_millis(500)).await;
    let _third = group.joined_member().await; // its join wakes the other members
    sleep_until(scheduled + DEFAULT_RENEW_INTERVAL + STEP).await;

    let renews_after: Vec<Duration> = second
        .adapter
        .call_times(Operation::Renew)
        .into_iter()
        .filter(|sent_at| *sent_at > scheduled)
        .map(|sent_at| sent_at - scheduled)
        .collect();
    assert_eq!(
        renews_after,
        vec![Duration::from_millis(500), DEFAULT_RENEW_INTERVAL]
    );
}

// spec: election-core / Master crashes
#[tokio::test(start_paused = true)]
async fn the_next_member_is_master_8_to_12_seconds_after_a_crash() {
    for offset_ms in [0, 300, 1_100, 1_900] {
        let group = TestGroup::new();
        let first = group.joined_member().await;
        let second = group.joined_member().await;
        let third = group.joined_member().await;
        first.wait_for_next_renew().await;
        sleep(Duration::from_millis(offset_ms)).await;

        drop(first); // a crash: no leave, no more renews
        let waited = wait_until(Duration::from_secs(20), || {
            second.status() == Status::Master
        })
        .await;

        assert!(
            waited >= Duration::from_secs(8) && waited <= Duration::from_secs(12),
            "offset {offset_ms} ms: master after {waited:?}"
        );
        assert_eq!(third.status(), Status::Slave);
    }
}

// spec: election-core / Short database problem
#[tokio::test(start_paused = true)]
async fn a_database_problem_of_5_seconds_changes_nothing() {
    for offset_ms in [0, 1_000, 1_900] {
        let group = TestGroup::new();
        let members = [
            group.joined_member().await,
            group.joined_member().await,
            group.joined_member().await,
        ];
        let roles_stay = || {
            members[0].status() == Status::Master
                && members[1].status() == Status::Slave
                && members[2].status() == Status::Slave
        };
        members[0].wait_for_next_renew().await;
        sleep(Duration::from_millis(offset_ms)).await;

        for member in &members {
            member.adapter.set_fault_on_every_call(Fault::NoAnswer);
        }
        let during = always_during(Duration::from_secs(5), roles_stay).await;
        for member in &members {
            member.adapter.clear_faults();
        }
        let after = always_during(Duration::from_secs(15), roles_stay).await;

        assert!(
            during,
            "offset {offset_ms} ms: a role changed during the problem"
        );
        assert!(
            after,
            "offset {offset_ms} ms: a role changed after the problem"
        );
    }
}

// spec: election-core / Watch stream ends
#[tokio::test(start_paused = true)]
async fn an_ended_watch_stream_is_opened_again() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;

    group.database.end_watch_streams(&group.name);
    wait_until(Duration::from_secs(5), || {
        second.adapter.call_count(Operation::Watch) == 2
    })
    .await;
    let slave_all_the_time =
        always_during(Duration::from_secs(3), || second.status() == Status::Slave).await;
    second.wait_for_next_renew().await;
    first.doyen.leave().await.expect("leave works");
    let waited = wait_until(Duration::from_secs(3), || second.status() == Status::Master).await;

    assert!(slave_all_the_time);
    assert!(
        waited < Duration::from_millis(500),
        "master after {waited:?}"
    );
}

// spec: election-core / Database lost the data
#[tokio::test(start_paused = true)]
async fn after_a_data_loss_the_member_joins_again() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let mut changes = member.doyen.status_changes();

    group.database.lose_all_data(&group.name).await;
    let mut notifications = Vec::new();
    wait_until(Duration::from_secs(5), || {
        notifications.extend(ready_notifications(&mut changes));
        notifications.last() == Some(&Status::Master)
    })
    .await;

    let reasons: Vec<Option<OfflineReason>> = notifications.iter().map(reason_of).collect();
    assert_eq!(reasons, vec![Some(OfflineReason::Reconnecting), None]);
    let number = member.number().expect("a new place");
    assert_eq!(group.live_numbers(), vec![number]);
}

// spec: election-core / Join cancelled after its write
#[tokio::test(start_paused = true)]
async fn a_new_join_after_a_cancel_finds_the_same_place() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_fault(Operation::Join, Fault::LostAnswer);
    let cancelled = timeout(Duration::from_secs(1), member.doyen.join(group.config())).await;
    assert!(cancelled.is_err(), "the join must still wait");
    let written = group.live_numbers();
    assert_eq!(written.len(), 1, "the cancelled join wrote its place");

    member.adapter.clear_faults();
    member.doyen.join(group.config()).await.expect("join works");

    assert_eq!(group.live_numbers(), written);
    assert_eq!(member.number(), Some(written[0]));
}

// spec: election-core / Master reads its token
#[tokio::test(start_paused = true)]
async fn the_master_token_holds_the_generation_and_the_number() {
    let group = TestGroup::new();

    let member = group.joined_member().await;

    let generation = group
        .database
        .stored_generation(&group.name)
        .expect("a generation");
    assert_eq!(
        member.doyen.token(),
        Some(Token {
            generation,
            number: 1
        })
    );
}

// spec: election-core / Lease ended
#[tokio::test(start_paused = true)]
async fn the_token_is_gone_at_once_when_the_lease_ends() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    member.wait_for_next_renew().await;
    let lease_end = last_renew(&member) + DEFAULT_LEASE;
    member.adapter.set_fault_on_every_call(Fault::NoAnswer);

    sleep_until(lease_end - STEP).await;
    assert!(member.doyen.token().is_some());
    sleep_until(lease_end).await;

    assert_eq!(member.doyen.token(), None);
}

// spec: election-core / Generation grows while master
#[tokio::test(start_paused = true)]
async fn a_new_generation_shows_in_the_token_without_a_notification() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let mut changes = member.doyen.status_changes();
    let before = member.doyen.token().expect("master");

    group.database.delete_generation_record(&group.name).await;
    member.wait_for_next_renew().await;

    let after = member.doyen.token().expect("still master");
    assert!(after.generation > before.generation);
    assert_eq!(after.number, before.number);
    assert_eq!(ready_notifications(&mut changes), Vec::<Status>::new());
}

// spec: election-core / Takeover in the same generation
#[tokio::test(start_paused = true)]
async fn the_token_of_the_next_master_is_higher() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    let old_token = first.doyen.token().expect("master");

    drop(first);
    wait_until(Duration::from_secs(15), || {
        second.status() == Status::Master
    })
    .await;

    let new_token = second.doyen.token().expect("master");
    assert_eq!(new_token.generation, old_token.generation);
    assert!(new_token > old_token);
}

// spec: election-core / New generation
#[tokio::test(start_paused = true)]
async fn every_token_of_a_new_generation_is_higher() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    let old_token = member.doyen.token().expect("master");

    group.database.delete_generation_record(&group.name).await;
    member.wait_for_next_renew().await;

    let new_token = member.doyen.token().expect("master");
    assert!(new_token.generation > old_token.generation);
    let highest_old_token = Token {
        generation: old_token.generation,
        number: u64::MAX,
    };
    assert!(new_token > highest_old_token);
}

// spec: election-core / Master leaves
#[tokio::test(start_paused = true)]
async fn a_leaving_master_stops_first_and_the_next_member_takes_over_at_once() {
    let group = TestGroup::new();
    let first = group.joined_member().await;
    let second = group.joined_member().await;
    first.wait_for_next_renew().await;
    first
        .adapter
        .set_fault(Operation::Leave, Fault::SlowAnswer(Duration::from_secs(1)));
    let doyen = first.doyen.clone();

    let leave = tokio::spawn(async move { doyen.leave().await });
    wait_until(Duration::from_millis(100), || {
        first.status() != Status::Master
    })
    .await;

    assert_eq!(first.doyen.token(), None);
    assert!(
        group.live_numbers().contains(&1),
        "the place is still there"
    );
    leave.await.expect("no panic").expect("leave works");
    let waited = wait_until(Duration::from_secs(3), || second.status() == Status::Master).await;
    assert!(
        waited < Duration::from_millis(500),
        "master after {waited:?}"
    );
}

// spec: election-core / Database down during leave
#[tokio::test(start_paused = true)]
async fn leave_finishes_after_one_call_timeout_when_the_database_is_down() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    member.wait_for_next_renew().await;
    let expiry = last_renew(&member) + DEFAULT_TIMEOUT;
    let number = member.number().expect("a place");
    member.adapter.set_fault_on_every_call(Fault::NoAnswer);

    let start = Instant::now();
    let result = member.doyen.leave().await;

    assert_eq!(result, Ok(()));
    assert_eq!(start.elapsed(), DEFAULT_RENEW_INTERVAL);
    sleep_until(expiry - STEP).await;
    assert_eq!(group.live_numbers(), vec![number]);
    sleep_until(expiry).await;
    assert_eq!(group.live_numbers(), Vec::<u64>::new());
}

// spec: election-core / Leave during join
#[tokio::test(start_paused = true)]
async fn leave_during_join_ends_the_join_with_doyen_left() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_fault_on_every_call(Fault::NoAnswer);
    let doyen = member.doyen.clone();
    let config = group.config();
    let join = tokio::spawn(async move { doyen.join(config).await });
    wait_until(Duration::from_secs(1), || {
        is_offline_with(&member.status(), OfflineReason::Connecting)
    })
    .await;

    member.doyen.leave().await.expect("leave works");
    let result = join.await.expect("no panic");

    assert_eq!(result.expect_err("join fails").code(), ErrorCode::Left);
    assert!(is_offline_with(&member.status(), OfflineReason::Left));
}

// spec: election-core / Join again on the same object
#[tokio::test(start_paused = true)]
async fn join_after_leave_fails_with_doyen_left() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    member.doyen.leave().await.expect("leave works");

    let result = member.doyen.join(group.config()).await;

    assert_eq!(result.expect_err("join fails").code(), ErrorCode::Left);
}

// spec: election-core / Join twice
#[tokio::test(start_paused = true)]
async fn a_second_join_fails_with_doyen_already_joined() {
    let group = TestGroup::new();
    let member = group.joined_member().await;

    let result = member.doyen.join(group.config()).await;

    assert_eq!(
        result.expect_err("join fails").code(),
        ErrorCode::AlreadyJoined
    );
    assert_eq!(member.status(), Status::Master);
}

#[tokio::test(start_paused = true)]
async fn a_second_leave_finishes_at_once() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    member.doyen.leave().await.expect("leave works");
    member.adapter.set_fault_on_every_call(Fault::NoAnswer);

    let start = Instant::now();
    let result = member.doyen.leave().await;

    assert_eq!(result, Ok(()));
    assert_eq!(start.elapsed(), Duration::ZERO);
}

// spec: election-core / Renew task stuck
#[tokio::test(start_paused = true)]
async fn the_status_is_offline_at_once_when_a_stuck_renew_passes_the_lease_end() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    member.wait_for_next_renew().await;
    let lease_end = last_renew(&member) + DEFAULT_LEASE;
    member.adapter.set_fault_on_every_call(Fault::NoAnswer);
    sleep_until(lease_end - STEP).await;
    assert_eq!(member.status(), Status::Master);

    // The clock passes the lease end while no task runs: no timer fired, no answer came.
    group.clock.suspend(STEP);

    assert!(is_offline_with(
        &member.status(),
        OfflineReason::ConnectionLost
    ));
}

// spec: election-core / Renew task panics
#[tokio::test(start_paused = true)]
async fn after_a_panic_nothing_renews_and_the_place_expires() {
    let group = TestGroup::new();
    let member = group.joined_member().await;
    member.wait_for_next_renew().await;
    let last_good_renew = last_renew(&member);
    let number = member.number().expect("a place");

    member.adapter.set_fault(Operation::Renew, Fault::Panic);
    member.wait_for_next_renew().await;
    let renews_after_the_panic = member.renew_count();
    sleep_until(last_good_renew + DEFAULT_LEASE).await;

    assert_ne!(member.status(), Status::Master);
    sleep_until(last_good_renew + DEFAULT_TIMEOUT).await;
    assert!(!group.live_numbers().contains(&number));
    sleep(Duration::from_secs(10)).await;
    assert_eq!(member.renew_count(), renews_after_the_panic);
}
