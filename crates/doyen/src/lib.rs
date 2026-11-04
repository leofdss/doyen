//! # doyen
//!
//! Pick one master among the instances of a service, through the database you already run.
//! The oldest live instance is the master; when it dies, the next in line takes over.
//!
//! **Work in progress:** this version only reserves the name. The first release (v0.1) brings
//! the Rust core with the Redis and MongoDB adapters. Follow
//! <https://github.com/leofdss/doyen>.
