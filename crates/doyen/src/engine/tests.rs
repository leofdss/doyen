// SPDX-License-Identifier: MIT OR Apache-2.0
//! The engine tests: the in-memory adapter, the fake clock and paused Tokio time.
//!
//! One file per capability. The timing tests read the numbers of the specs in test time:
//! a wait of 10 s takes no real time.

mod backoff;
mod binding_hooks;
mod client_config;
mod election_core;
mod helpers;
mod rust_api;
mod status_reporting;
