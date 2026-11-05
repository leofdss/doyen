// SPDX-License-Identifier: MIT OR Apache-2.0
//! # doyen
//!
//! Pick one master among the instances of a service, through the database you already run.
//! The oldest live instance is the master; when it dies, the next in line takes over.
//!
//! **Work in progress:** this version only reserves the name. The first release (v0.1) brings
//! the Rust core with the Redis and MongoDB adapters. Follow
//! <https://github.com/leofdss/doyen>.

// Failure paths of the CI (task 5.2 of add-ci-baseline). Throwaway branch, never merged.

pub fn unformatted( ) -> u32 { 1 }

/// Clippy warns: `&Vec<u32>` should be `&[u32]` (clippy::ptr_arg).
pub fn clippy_warning(values: &Vec<u32>) -> usize {
    values.len()
}

/// `Vec::pop_if` is stable since Rust 1.86, so the msrv job (Rust 1.85) fails.
pub fn needs_newer_rust(values: &mut Vec<u32>) -> Option<u32> {
    values.pop_if(|value| *value > 1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_test_fails() {
        let total: u32 = [1, 1].iter().sum();
        assert_eq!(total, 3);
    }
}
