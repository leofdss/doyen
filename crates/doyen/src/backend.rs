// SPDX-License-Identifier: MIT OR Apache-2.0
//! The contract between the election engine and a database adapter.
//!
//! An adapter only stores and reads: numbers, places, the live line and the generation. The
//! master rule stays in the engine (spec adapter-contract, "Five operations").

// The engine (change task group 6) uses these types; until then only the tests do.
#![cfg_attr(not(test), allow(dead_code))]

use std::time::Duration;

use async_trait::async_trait;
use futures_util::stream::BoxStream;

use crate::error::{Error, ErrorCode};

/// The data format that this version of doyen writes with the group data.
pub(crate) const DATA_FORMAT: u32 = 1;

/// The answer of an adapter call.
pub(crate) type Result<T> = std::result::Result<T, BackendError>;

/// Why an adapter call failed.
///
/// The message never holds a secret: an adapter shows a URL only through `redact_url`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum BackendError {
    /// The call may work later: a network error, a timeout or a failover. The engine
    /// retries it.
    #[error("{0}")]
    Network(String),
    /// The database refused the setup, the login or a command (`DOYEN_AUTH`,
    /// `DOYEN_UNSUPPORTED`). Before the first place, `join` ends with this error; after it,
    /// the engine retries it like a network error (spec election-core, "After join the core
    /// never gives up").
    #[error(transparent)]
    Refused(Error),
}

/// The error for group data in a format newer than [`DATA_FORMAT`]. Every adapter uses it,
/// so the message is the same for every database (spec adapter-contract, "Data format of a
/// group").
pub(crate) fn newer_format_error(stored_format: u32) -> BackendError {
    BackendError::Refused(Error::new(
        ErrorCode::Unsupported,
        format!(
            "the group data has format {stored_format}, but this version of doyen knows only \
             format {DATA_FORMAT}: update doyen on every instance of the group"
        ),
    ))
}

/// One place in the line: the number and the tag of its owner.
///
/// A member is its number, never its instance name. The tag proves that the place is
/// still mine: after a data loss the database can give my number to someone else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Membership {
    /// The seniority number that `join` gave.
    pub(crate) number: u64,
    /// The random 128-bit tag that `join` stored with the place.
    pub(crate) tag: u128,
}

/// What the database says about a group: only data, no decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct View {
    /// The generation of the group. It never goes back.
    pub(crate) generation: u64,
    /// The numbers of the live members, in increasing order. The engine, not the adapter,
    /// decides that the first one is the master.
    pub(crate) line: Vec<u64>,
}

/// A setup that works but has a known risk, found by `connect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Warning {
    /// The code, for example `DOYEN_REDIS_EVICTION`. Each adapter defines its own codes.
    pub(crate) code: String,
    /// What the risk is and how to fix it.
    pub(crate) message: String,
}

/// A database adapter: the five operations of the spec adapter-contract.
///
/// Every call must be safe to cancel at any point (the engine wraps each one in a
/// timeout).
// `async_trait` because the engine holds a `Box<dyn Backend>`: the database is chosen at run
// time from the config, and a native `async fn` in a trait cannot be used through `dyn`.
#[async_trait]
pub(crate) trait Backend: Send + Sync + 'static {
    /// Check the database setup (version, topology, settings). Fails with a refused error
    /// for a setup doyen does not accept, and gives the warnings for a setup with a known
    /// risk. Writes no group data.
    async fn connect(&self) -> Result<Vec<Warning>>;

    /// Get a number and a place in the group, in one atomic step. A live place with the same
    /// tag is returned again, never a second one. `id` is the instance name: only a label
    /// for the database records, never the key of a member.
    async fn join(&self, group: &str, id: &str, tag: u128, ttl: Duration) -> Result<(u64, View)>;

    /// Keep my place alive for `ttl` more, and read the live line. `None` means "place
    /// gone": it expired, or it has another tag. A renew never creates a place.
    async fn renew(&self, group: &str, me: &Membership, ttl: Duration) -> Result<Option<View>>;

    /// Remove my place (only when it has my tag) and wake the other members.
    async fn leave(&self, group: &str, me: &Membership) -> Result<()>;

    /// A stream with one item each time someone joins or leaves the group. An item carries
    /// no data: the engine reads the line again with `renew`. `None` means the database has
    /// no push, and the engine uses only its timer.
    async fn watch(&self, group: &str) -> Result<Option<BoxStream<'static, ()>>>;
}
