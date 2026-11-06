// SPDX-License-Identifier: MIT OR Apache-2.0
//! The conformance suite: the rules of the spec adapter-contract, for every adapter.
//!
//! Each rule is an async function, generic over [`TestDatabase`], that panics when the
//! adapter breaks the rule. An adapter runs the whole suite with one line:
//! `conformance_tests!(start_function);`, where `start_function` is an async function that
//! returns `Some(database)`, or `None` to skip the tests (see [`database_url`]).
//!
//! The same rules run on the in-memory adapter (fake clock, paused Tokio time) and on a real
//! database (real time). The waits use `tokio::time::sleep`, which does both: in paused time
//! it moves the clock at once.

use std::time::Duration;

use futures_util::StreamExt;

use crate::backend::{Backend, BackendError, Membership, View};
use crate::error::ErrorCode;

#[cfg(test)]
mod tests;

/// A database the suite runs on, with the test hooks of its adapter. The hooks write to the
/// database directly, like a person or an outage would.
pub(crate) trait TestDatabase {
    /// The adapter under test.
    type Adapter: Backend;

    /// A new adapter on this database: one more app instance.
    async fn new_adapter(&self) -> Self::Adapter;

    /// A group name for one test. On a shared database it must be unique, so tests that run
    /// at the same time do not meet.
    fn group_name(&self, name: &str) -> String;

    /// Delete all doyen data of the group, like a restart without persistence.
    async fn lose_all_data(&self, group: &str);

    /// Delete the record that holds the generation of the group, and nothing else when the
    /// database keeps it apart.
    async fn delete_generation_record(&self, group: &str);

    /// Set the number counter of the group to `value`, like a restore of an old backup.
    async fn set_counter(&self, group: &str, value: u64);

    /// Write the data format `format` in the group data (a newer doyen did it).
    async fn write_format(&self, group: &str, format: u32);

    /// An adapter on a server older than the adapter's minimum version, and that minimum
    /// version as the message must show it.
    async fn adapter_on_old_server(&self) -> (Self::Adapter, String);
}

/// The variable that makes a missing test database a failure (set in CI).
const REQUIRE_DATABASE_VARIABLE: &str = "DOYEN_TEST_REQUIRE_DB";

/// The database test switch: the URL of a test database, read from `variable` (for example
/// `DOYEN_TEST_REDIS_URL`).
///
/// `None` means "skip the database tests": the variable is not set and
/// `DOYEN_TEST_REQUIRE_DB` is not `1`. With `DOYEN_TEST_REQUIRE_DB=1` a missing URL panics,
/// so a CI job never skips its database tests without a sign.
#[expect(
    dead_code,
    reason = "the test start functions of the Redis and MongoDB adapters call it"
)]
pub(crate) fn database_url(variable: &str) -> Option<String> {
    let url = std::env::var(variable).ok();
    let require_database = std::env::var(REQUIRE_DATABASE_VARIABLE).ok();
    match choose_database_url(variable, url, require_database) {
        Ok(Some(url)) => Some(url),
        Ok(None) => {
            // Only the name of the variable: the URL may hold a password.
            eprintln!("skipped: {variable} is not set");
            None
        }
        Err(message) => panic!("{message}"),
    }
}

/// The decision of [`database_url`], from the values of the two variables. An empty URL
/// counts as missing: a CI secret that is not set gives an empty variable.
fn choose_database_url(
    variable: &str,
    url: Option<String>,
    require_database: Option<String>,
) -> Result<Option<String>, String> {
    let url = url.filter(|url| !url.is_empty());
    if url.is_some() {
        return Ok(url);
    }
    if require_database.as_deref() == Some("1") {
        return Err(format!(
            "{variable} is not set, and {REQUIRE_DATABASE_VARIABLE}=1 requires the database \
             tests: set {variable} to the URL of a test database"
        ));
    }
    Ok(None)
}

/// Write one test per rule of the conformance suite, for one adapter.
///
/// `conformance_tests!(start)` checks that the adapter follows every rule.
/// `conformance_tests!(start, every_rule_fails)` checks that every rule fails, for an adapter
/// that is broken on purpose.
// A macro, because each adapter must run the same list of tests and a `#[test]` function
// cannot be generic. Without it, each adapter would copy the whole list by hand and could
// miss a rule.
macro_rules! conformance_tests {
    // One test: `$extra` holds the attributes to add, for example `#[should_panic]`.
    (@test $start:path, [$($extra:tt)*], $rule:ident) => {
        #[tokio::test]
        $($extra)*
        async fn $rule() {
            let Some(database) = $start().await else {
                return;
            };
            $crate::conformance::$rule(&database).await;
        }
    };
    (@tests $start:path, $extra:tt) => {
        $crate::conformance::conformance_tests!(
            @each_rule $start,
            $extra,
            adapter_decides_nothing,
            instance_comes_back_with_the_same_name,
            number_given_twice,
            two_joins_at_the_same_time,
            counter_went_back,
            retry_after_a_lost_answer,
            generation_record_deleted,
            first_join_after_a_restart,
            app_clock_is_wrong,
            renew_after_expiry,
            a_member_expired,
            lost_wake_up,
            join_and_leave_wake_the_watchers,
            join_cancelled_by_a_timeout,
            old_server_version,
            newer_format_at_join,
            newer_format_while_running,
        );
    };
    (@each_rule $start:path, $extra:tt, $($rule:ident,)*) => {
        $(
            $crate::conformance::conformance_tests!(@test $start, $extra, $rule);
        )*
    };
    ($start:path, every_rule_fails) => {
        $crate::conformance::conformance_tests!(@tests $start, [#[should_panic]]);
    };
    ($start:path) => {
        $crate::conformance::conformance_tests!(@tests $start, []);
    };
}

pub(crate) use conformance_tests;

/// The `ttl` of the places in the tests.
const TTL: Duration = Duration::from_secs(3);
/// The `ttl` of a member that only watches the line during a test: it never expires there.
const OBSERVER_TTL: Duration = Duration::from_secs(600);
/// How long a test waits for a wake-up.
const WAKE_UP_WAIT: Duration = Duration::from_secs(2);

const TAG_A: u128 = 0xA000_0000_0000_0000_0000_0000_0000_000A;
const TAG_B: u128 = 0xB000_0000_0000_0000_0000_0000_0000_000B;
const TAG_C: u128 = 0xC000_0000_0000_0000_0000_0000_0000_000C;
const TAG_OBSERVER: u128 = 0x0B5E_0000_0000_0000_0000_0000_0000_0B5E;

/// Join and return the membership with its view. Panics on an error.
async fn join_member(
    adapter: &impl Backend,
    group: &str,
    tag: u128,
    ttl: Duration,
) -> (Membership, View) {
    let (number, view) = adapter
        .join(group, "test-instance", tag, ttl)
        .await
        .expect("join must work");
    (Membership { number, tag }, view)
}

/// Renew with the test `ttl` and return the view, or `None` for "place gone". Panics on an
/// error.
async fn renew_member(adapter: &impl Backend, group: &str, me: &Membership) -> Option<View> {
    adapter
        .renew(group, me, TTL)
        .await
        .expect("renew must work")
}

/// The live line as a member that never expires in the test sees it.
async fn live_line(adapter: &impl Backend, group: &str) -> Vec<u64> {
    let (observer, _) = join_member(adapter, group, TAG_OBSERVER, OBSERVER_TTL).await;
    let view = renew_member(adapter, group, &observer)
        .await
        .expect("the observer must keep its place");
    view.line
        .into_iter()
        .filter(|number| *number != observer.number)
        .collect()
}

fn assert_increasing(line: &[u64]) {
    let is_increasing = line.windows(2).all(|pair| pair[0] < pair[1]);
    assert!(
        is_increasing,
        "the line must be in increasing order: {line:?}"
    );
}

/// The code of a refused error. Panics for a network error.
fn refused_code(error: &BackendError) -> ErrorCode {
    match error {
        BackendError::Refused(refused) => refused.code(),
        BackendError::Network(message) => panic!("expected a refused error, got {message}"),
    }
}

// spec: adapter-contract / Adapter decides nothing
pub(crate) async fn adapter_decides_nothing(database: &impl TestDatabase) {
    let group = database.group_name("adapter-decides-nothing");
    let adapters = [
        database.new_adapter().await,
        database.new_adapter().await,
        database.new_adapter().await,
    ];
    let tags = [TAG_A, TAG_B, TAG_C];
    let mut members = Vec::new();
    for (adapter, tag) in adapters.iter().zip(tags) {
        let (member, _) = join_member(adapter, &group, tag, TTL).await;
        members.push(member);
    }

    // The view holds only the line and the generation: every member gets the same data,
    // and the engine takes the first number of the line as the master.
    let mut views = Vec::new();
    for (adapter, member) in adapters.iter().zip(&members) {
        let view = renew_member(adapter, &group, member)
            .await
            .expect("a live member keeps its place");
        views.push(view);
    }
    let numbers: Vec<u64> = members.iter().map(|member| member.number).collect();
    for view in &views {
        assert_eq!(view, &views[0], "every member must see the same view");
        assert_eq!(view.line, numbers);
    }
}

// spec: adapter-contract / Instance comes back with the same name
pub(crate) async fn instance_comes_back_with_the_same_name(database: &impl TestDatabase) {
    let group = database.group_name("instance-comes-back");
    let pod_a = database.new_adapter().await;
    let (dead, _) = join_member(&pod_a, &group, TAG_A, TTL).await;
    tokio::time::sleep(TTL + Duration::from_secs(1)).await;

    let (alive, _) = join_member(&pod_a, &group, TAG_B, TTL).await;
    assert!(alive.number > dead.number);
    pod_a
        .leave(&group, &dead)
        .await
        .expect("leave of a dead place must work");

    let view = renew_member(&pod_a, &group, &alive)
        .await
        .expect("removing the dead number must not touch the new membership");
    assert!(view.line.contains(&alive.number));
}

// spec: adapter-contract / Number given twice
pub(crate) async fn number_given_twice(database: &impl TestDatabase) {
    let group = database.group_name("number-given-twice");
    let adapter_a = database.new_adapter().await;
    let adapter_b = database.new_adapter().await;
    let (member_a, _) = join_member(&adapter_a, &group, TAG_A, TTL).await;
    database.lose_all_data(&group).await;
    let (member_b, _) = join_member(&adapter_b, &group, TAG_B, TTL).await;

    let view_a = renew_member(&adapter_a, &group, &member_a).await;
    let view_b = renew_member(&adapter_b, &group, &member_b).await;
    if member_a.number == member_b.number {
        assert!(
            view_a.is_none() || view_b.is_none(),
            "two members must never share one place"
        );
    }
    assert!(
        view_b.is_some(),
        "the member that joined last keeps its place"
    );
    for view in [&view_a, &view_b].into_iter().flatten() {
        assert_increasing(&view.line);
    }

    // The member that lost the place joins again and gets a place of its own.
    if view_a.is_none() {
        let (new_member_a, view) = join_member(&adapter_a, &group, TAG_C, TTL).await;
        assert_ne!(new_member_a.number, member_b.number);
        assert_increasing(&view.line);
        assert!(view.line.contains(&member_b.number));
        assert!(view.line.contains(&new_member_a.number));
    }
}

// spec: adapter-contract / Two joins at the same time
pub(crate) async fn two_joins_at_the_same_time(database: &impl TestDatabase) {
    let group = database.group_name("two-joins");
    let adapter_a = database.new_adapter().await;
    let adapter_b = database.new_adapter().await;

    let (joined_a, joined_b) = tokio::join!(
        join_member(&adapter_a, &group, TAG_A, TTL),
        join_member(&adapter_b, &group, TAG_B, TTL),
    );
    let (member_a, member_b) = (joined_a.0, joined_b.0);
    assert_ne!(member_a.number, member_b.number);

    let view = renew_member(&adapter_a, &group, &member_a)
        .await
        .expect("a live member keeps its place");
    let mut expected = vec![member_a.number, member_b.number];
    expected.sort();
    assert_eq!(view.line, expected);
}

// spec: adapter-contract / Counter went back
pub(crate) async fn counter_went_back(database: &impl TestDatabase) {
    let group = database.group_name("counter-went-back");
    let adapter = database.new_adapter().await;
    let mut highest_number = 0;
    for tag in 1..=5 {
        let (member, _) = join_member(&adapter, &group, tag, TTL).await;
        highest_number = member.number;
    }
    database.set_counter(&group, 2).await;

    // A new instance: it has no memory of the numbers it saw.
    let new_adapter = database.new_adapter().await;
    let (new_member, view) = join_member(&new_adapter, &group, TAG_A, TTL).await;
    assert!(
        new_member.number > highest_number,
        "{} must be above {highest_number}",
        new_member.number
    );
    assert_increasing(&view.line);
}

// spec: adapter-contract / Retry after a lost answer
pub(crate) async fn retry_after_a_lost_answer(database: &impl TestDatabase) {
    let group = database.group_name("retry-after-a-lost-answer");
    let adapter = database.new_adapter().await;
    let (first, _) = join_member(&adapter, &group, TAG_A, TTL).await;

    let (retry, view) = join_member(&adapter, &group, TAG_A, TTL).await;
    assert_eq!(retry.number, first.number);
    assert_eq!(view.line, vec![first.number], "one place for the tag");
}

/// The generation an adapter gives next: from a renew, or from a new join when the place is
/// gone (a database that keeps the generation with the places loses both together).
async fn next_generation(adapter: &impl Backend, group: &str, me: &Membership) -> u64 {
    match renew_member(adapter, group, me).await {
        Some(view) => view.generation,
        None => {
            let (_, view) = join_member(adapter, group, TAG_C, TTL).await;
            view.generation
        }
    }
}

// spec: adapter-contract / Generation record deleted
pub(crate) async fn generation_record_deleted(database: &impl TestDatabase) {
    let group = database.group_name("generation-record-deleted");
    let adapter_a = database.new_adapter().await;
    let adapter_b = database.new_adapter().await;
    let (member_a, _) = join_member(&adapter_a, &group, TAG_A, TTL).await;
    let (_, view_b) = join_member(&adapter_b, &group, TAG_B, TTL).await;
    let old_generation = view_b.generation;

    database.delete_generation_record(&group).await;

    let generation = next_generation(&adapter_a, &group, &member_a).await;
    assert!(
        generation > old_generation,
        "{generation} must be above {old_generation}"
    );
}

// spec: adapter-contract / First join after a restart
pub(crate) async fn first_join_after_a_restart(database: &impl TestDatabase) {
    let group = database.group_name("first-join-after-a-restart");
    // A running instance: it saw the generation before the restart.
    let adapter = database.new_adapter().await;
    let (_, view_before) = join_member(&adapter, &group, TAG_A, TTL).await;

    database.lose_all_data(&group).await;

    let (_, view_after) = join_member(&adapter, &group, TAG_B, TTL).await;
    assert!(
        view_after.generation > view_before.generation,
        "{} must be above {}",
        view_after.generation,
        view_before.generation
    );
}

// spec: adapter-contract / App clock is wrong
pub(crate) async fn app_clock_is_wrong(database: &impl TestDatabase) {
    // A test cannot move the clock of the app machine. The adapter calls carry no app
    // time (only `ttl`), so this test checks the other half of the rule: a place lives
    // `ttl` from its write, by the database's own time.
    let group = database.group_name("app-clock-is-wrong");
    let adapter = database.new_adapter().await;
    let (member, _) = join_member(&adapter, &group, TAG_A, TTL).await;

    tokio::time::sleep(TTL - Duration::from_millis(500)).await;
    assert!(live_line(&adapter, &group).await.contains(&member.number));

    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!live_line(&adapter, &group).await.contains(&member.number));
}

// spec: adapter-contract / Renew after expiry
pub(crate) async fn renew_after_expiry(database: &impl TestDatabase) {
    let group = database.group_name("renew-after-expiry");
    let adapter = database.new_adapter().await;
    let (member, _) = join_member(&adapter, &group, TAG_A, TTL).await;
    tokio::time::sleep(TTL + Duration::from_secs(1)).await;

    let view = renew_member(&adapter, &group, &member).await;
    assert_eq!(view, None, "the place is gone");
    assert!(
        !live_line(&adapter, &group).await.contains(&member.number),
        "the renew must not create the place again"
    );
}

// spec: adapter-contract / A member expired
pub(crate) async fn a_member_expired(database: &impl TestDatabase) {
    let group = database.group_name("a-member-expired");
    let adapters = [
        database.new_adapter().await,
        database.new_adapter().await,
        database.new_adapter().await,
    ];
    let (first, _) = join_member(&adapters[0], &group, TAG_A, TTL).await;
    let (second, _) = join_member(&adapters[1], &group, TAG_B, TTL).await;
    let (third, _) = join_member(&adapters[2], &group, TAG_C, TTL).await;
    assert!(first.number < second.number && second.number < third.number);

    // The first and the third member renew; the second one stops and expires.
    tokio::time::sleep(TTL - Duration::from_secs(1)).await;
    for (adapter, member) in [(&adapters[0], &first), (&adapters[2], &third)] {
        renew_member(adapter, &group, member)
            .await
            .expect("a live member keeps its place");
    }
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let view = renew_member(&adapters[0], &group, &first)
        .await
        .expect("a live member keeps its place");
    assert_eq!(view.line, vec![first.number, third.number]);
}

// spec: adapter-contract / Lost wake-up
pub(crate) async fn lost_wake_up(database: &impl TestDatabase) {
    let group = database.group_name("lost-wake-up");
    let adapter_a = database.new_adapter().await;
    let adapter_b = database.new_adapter().await;
    let (member_a, _) = join_member(&adapter_a, &group, TAG_A, TTL).await;
    let (member_b, _) = join_member(&adapter_b, &group, TAG_B, TTL).await;
    // Member A opens its stream but never reads it: the wake-up of the leave is lost.
    let _unread_stream = adapter_a.watch(&group).await.expect("watch must work");

    adapter_b
        .leave(&group, &member_b)
        .await
        .expect("leave must work");

    let view = renew_member(&adapter_a, &group, &member_a)
        .await
        .expect("a live member keeps its place");
    assert_eq!(view.line, vec![member_a.number]);
}

/// Not a scenario of its own: the wake-ups of the requirement "Wake-ups carry no data", which
/// the engine needs for a fast takeover after a leave.
pub(crate) async fn join_and_leave_wake_the_watchers(database: &impl TestDatabase) {
    let group = database.group_name("wake-ups");
    let adapter_a = database.new_adapter().await;
    let adapter_b = database.new_adapter().await;
    join_member(&adapter_a, &group, TAG_A, TTL).await;
    let Some(mut wake_ups) = adapter_a.watch(&group).await.expect("watch must work") else {
        // The database has no push: the engine uses only its timer.
        return;
    };

    let (member_b, _) = join_member(&adapter_b, &group, TAG_B, TTL).await;
    let after_join = tokio::time::timeout(WAKE_UP_WAIT, wake_ups.next()).await;
    assert_eq!(after_join, Ok(Some(())), "a join wakes the watchers");

    adapter_b
        .leave(&group, &member_b)
        .await
        .expect("leave must work");
    let after_leave = tokio::time::timeout(WAKE_UP_WAIT, wake_ups.next()).await;
    assert_eq!(after_leave, Ok(Some(())), "a leave wakes the watchers");
}

// spec: adapter-contract / Join cancelled by a timeout
pub(crate) async fn join_cancelled_by_a_timeout(database: &impl TestDatabase) {
    let group = database.group_name("join-cancelled");
    let adapter = database.new_adapter().await;
    let observer = database.new_adapter().await;

    // The join reaches the database, and the timeout ends the call before its answer
    // comes back. No retry follows.
    let join_with_a_slow_answer = async {
        let answer = adapter.join(&group, "test-instance", TAG_A, TTL).await;
        tokio::time::sleep(Duration::from_secs(60)).await;
        answer
    };
    let cancelled = tokio::time::timeout(Duration::from_secs(2), join_with_a_slow_answer).await;
    assert!(cancelled.is_err(), "the call must be cancelled");

    let line_after_the_cancel = live_line(&observer, &group).await;
    assert_eq!(line_after_the_cancel.len(), 1, "the join wrote one place");

    tokio::time::sleep(TTL).await;
    assert_eq!(live_line(&observer, &group).await, Vec::<u64>::new());
}

// spec: adapter-contract / Old server version
pub(crate) async fn old_server_version(database: &impl TestDatabase) {
    let (adapter, minimum_version) = database.adapter_on_old_server().await;

    let error = adapter
        .connect()
        .await
        .expect_err("connect must refuse an old server");
    assert_eq!(refused_code(&error), ErrorCode::Unsupported);
    assert!(
        error.to_string().contains(&minimum_version),
        "the message must name the minimum version {minimum_version}: {error}"
    );
}

// spec: adapter-contract / Newer format at join
pub(crate) async fn newer_format_at_join(database: &impl TestDatabase) {
    let group = database.group_name("newer-format-at-join");
    let adapter = database.new_adapter().await;
    database.write_format(&group, 2).await;

    let error = adapter
        .join(&group, "test-instance", TAG_A, TTL)
        .await
        .expect_err("join must refuse a newer format");
    assert_eq!(refused_code(&error), ErrorCode::Unsupported);
    let message = error.to_string();
    assert!(message.contains("format 2"), "{message}");
    assert!(message.contains("format 1"), "{message}");
}

// spec: adapter-contract / Newer format while running
pub(crate) async fn newer_format_while_running(database: &impl TestDatabase) {
    let group = database.group_name("newer-format-while-running");
    let adapter = database.new_adapter().await;
    let (member, _) = join_member(&adapter, &group, TAG_A, TTL).await;
    database.write_format(&group, 2).await;

    // An error, not "place gone": the engine keeps the place and goes offline when its
    // lease ends, instead of joining again in a group it cannot read.
    let first_renew = adapter.renew(&group, &member, TTL).await;
    assert!(first_renew.is_err(), "renew must fail: {first_renew:?}");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let second_renew = adapter.renew(&group, &member, TTL).await;
    assert!(second_renew.is_err(), "renew must fail: {second_renew:?}");

    // An adapter may still refresh its place in the first renew that sees the new format
    // (the read and the write go in one round trip), never after. So the place is gone
    // `ttl` after the first renew, before the end of a refresh by the second one.
    tokio::time::sleep(TTL - Duration::from_millis(500)).await;
    database.write_format(&group, 1).await;
    let view = renew_member(&adapter, &group, &member).await;
    assert_eq!(
        view, None,
        "the adapter wrote nothing after it saw format 2"
    );
}
