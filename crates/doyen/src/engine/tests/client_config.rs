// SPDX-License-Identifier: MIT OR Apache-2.0
//! The scenarios of the spec client-config that need the engine.

use std::time::Duration;

use super::helpers::*;
use crate::error::{Error, ErrorCode};
use crate::memory::{Fault, Operation};
use crate::status::{OfflineReason, Status};

// spec: client-config / Two groups
#[tokio::test(start_paused = true)]
async fn each_group_has_its_own_master() {
    let billing = TestGroup::named("billing");
    let mail = billing.other_group("mail");

    let billing_first = billing.joined_member().await;
    let mail_first = mail.joined_member().await;
    let billing_second = billing.joined_member().await;
    let mail_second = mail.joined_member().await;
    billing_first.wait_for_next_renew().await;

    assert_eq!(billing_first.status(), Status::Master);
    assert_eq!(billing_second.status(), Status::Slave);
    assert_eq!(mail_first.status(), Status::Master);
    assert_eq!(mail_second.status(), Status::Slave);
    assert_eq!(billing.live_numbers(), vec![1, 2]);
    assert_eq!(mail.live_numbers(), vec![1, 2]);
}

// spec: client-config / Refused command
#[tokio::test(start_paused = true)]
async fn a_refused_command_ends_join_with_doyen_auth() {
    let group = TestGroup::new();
    let member = group.member();
    let refused = Error::new(
        ErrorCode::Auth,
        "the database refused the command EVALSHA: NOPERM this user has no permissions to \
         run the 'evalsha' command",
    );
    member
        .adapter
        .set_fault(Operation::Join, Fault::Refused(refused));

    let result = member.doyen.join(group.config()).await;

    let error = result.expect_err("join fails");
    assert_eq!(error.code(), ErrorCode::Auth);
    assert!(error.message().contains("EVALSHA"), "{error}");
}

// spec: client-config / Network error
#[tokio::test(start_paused = true)]
async fn a_network_error_at_join_is_retried_and_shown_in_the_status() {
    let group = TestGroup::new();
    let member = group.member();
    member.adapter.set_fault(
        Operation::Connect,
        Fault::NetworkError("connection refused".to_owned()),
    );
    let doyen = member.doyen.clone();
    let config = group.config();
    let join = tokio::spawn(async move { doyen.join(config).await });

    wait_until(Duration::from_secs(1), || {
        let status = member.status();
        let error = error_of(&status).unwrap_or_default();
        is_offline_with(&status, OfflineReason::Connecting) && error.contains("connection refused")
    })
    .await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert!(
        !join.is_finished(),
        "join must not return the network error"
    );
    assert!(
        member.adapter.call_count(Operation::Connect) > 2,
        "no retry"
    );

    member.adapter.clear_faults();
    let result = join.await.expect("no panic");

    assert_eq!(result, Ok(()));
    assert_eq!(member.status(), Status::Master);
}
