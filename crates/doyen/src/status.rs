// SPDX-License-Identifier: MIT OR Apache-2.0
//! The status of an instance and the reasons for being offline.

use std::fmt;

#[cfg(test)]
mod tests;

/// What an instance reports about itself.
///
/// ```
/// use doyen::{OfflineReason, Status};
///
/// let status = Status::Offline { reason: OfflineReason::NotJoined, error: None };
/// assert!(matches!(status, Status::Offline { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// This instance is the master of its group.
    Master,
    /// Another instance is the master.
    Slave,
    /// This instance has no place now: it is neither master nor slave.
    Offline {
        /// Why the instance is offline.
        reason: OfflineReason,
        /// The message of the last failed database call, when there is one.
        error: Option<String>,
    },
}

/// Why an instance is offline.
///
/// `Display` gives the camelCase name, the same text in every language.
///
/// ```
/// use doyen::OfflineReason;
///
/// assert_eq!(OfflineReason::ConnectionLost.to_string(), "connectionLost");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OfflineReason {
    /// `join` was not called, failed, or was cancelled.
    NotJoined,
    /// `join` runs and there is no place yet.
    Connecting,
    /// The lease ended; the core keeps trying to renew the same place.
    ConnectionLost,
    /// The place is gone; the core joins again.
    Reconnecting,
    /// `leave` finished. This is the last status.
    Left,
}

impl fmt::Display for OfflineReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            OfflineReason::NotJoined => "notJoined",
            OfflineReason::Connecting => "connecting",
            OfflineReason::ConnectionLost => "connectionLost",
            OfflineReason::Reconnecting => "reconnecting",
            OfflineReason::Left => "left",
        };
        formatter.write_str(name)
    }
}
