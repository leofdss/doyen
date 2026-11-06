// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tests of the status types.

use super::*;

// spec: rust-api / Display a reason
#[test]
fn a_reason_displays_its_camel_case_name() {
    assert_eq!(OfflineReason::NotJoined.to_string(), "notJoined");
    assert_eq!(OfflineReason::Connecting.to_string(), "connecting");
    assert_eq!(OfflineReason::ConnectionLost.to_string(), "connectionLost");
    assert_eq!(OfflineReason::Reconnecting.to_string(), "reconnecting");
    assert_eq!(OfflineReason::Left.to_string(), "left");
}

// spec: status-reporting / Log a reason
#[test]
fn the_reason_of_an_offline_status_prints_as_text() {
    let status = Status::Offline {
        reason: OfflineReason::ConnectionLost,
        error: Some("READONLY".to_owned()),
    };

    let Status::Offline { reason, .. } = status else {
        panic!("the status must be offline");
    };

    assert_eq!(format!("reason={reason}"), "reason=connectionLost");
}
