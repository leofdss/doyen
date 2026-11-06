// SPDX-License-Identifier: MIT OR Apache-2.0
//! # doyen
//!
//! Pick one master among the instances of a service, through the database you already run.
//! The oldest live instance is the master; when it dies, the next in line takes over.
//!
//! **Work in progress:** this version only reserves the name. The first release (v0.1) brings
//! the Rust core with the Redis and MongoDB adapters. Follow
//! <https://github.com/leofdss/doyen>.

#![deny(missing_docs)]

mod backend;
mod clock;
mod config;
#[cfg(test)]
mod conformance;
mod engine;
mod error;
#[cfg(test)]
mod memory;
mod status;
mod token;

pub use config::{DatabaseConfig, MongodbConfig, RedisConfig};
pub use engine::Doyen;
// The hooks of the bindings (not the public API). The module lives inside `engine`, so it
// can reach the private parts of `Doyen`.
#[doc(hidden)]
pub use engine::binding as __binding;
pub use error::{Error, ErrorCode};
pub use status::{OfflineReason, Status};
pub use token::Token;
