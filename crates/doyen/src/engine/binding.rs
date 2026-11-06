// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hooks for the Node.js and Go bindings only. This module is hidden and is not part of the
//! public API: it can change in any version, with no semver promise.

use std::sync::Arc;

use futures_core::Stream;

use super::Doyen;
pub use crate::backend::Warning;

/// Set the liveness check of `doyen` (a new call replaces the last check). It can be set
/// before or after `join`.
///
/// The driver calls it before each renew; when it returns `false`, that renew is skipped.
/// When the lease ends with no renew, the status is offline `connectionLost`; when the check
/// returns `true` again, the next renew gets the place back, if it did not expire. The
/// Node.js binding returns `false` when no tick ran in the last 2 × `renewIntervalMs` (a
/// blocked event loop). The check must be quick: it runs on the driver task.
pub fn set_liveness(doyen: &Doyen, check: impl Fn() -> bool + Send + Sync + 'static) {
    doyen.handle.shared.set_liveness_check(Arc::new(check));
}

/// A new stream of the connect warnings of `doyen`, from now on: each warning that
/// `connect` gives comes once, before `join` finishes. It does not repeat the warnings that
/// came before the stream, and it ends after `leave`.
///
/// Each stream has its own queue with no limit. Rust apps get the warnings through
/// `tracing` only; the Node.js binding sends them to `process.emitWarning`, the Go binding
/// to `slog`.
pub fn warnings(doyen: &Doyen) -> impl Stream<Item = Warning> + Send + Unpin + 'static {
    let mut warnings = doyen.handle.shared.subscribe_to_warnings();
    futures_util::stream::poll_fn(move |context| warnings.poll_recv(context))
}
