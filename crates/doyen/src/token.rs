// SPDX-License-Identifier: MIT OR Apache-2.0
//! The fencing token.

#[cfg(test)]
mod tests;

/// The fencing token of a master.
///
/// A higher token is newer. A resource that checks tokens rejects a write with a lower
/// token than the highest it has seen, so an old master cannot write.
///
/// ```
/// use doyen::Token;
///
/// let new_master = Token { generation: 2, number: 1 };
/// let old_master = Token { generation: 1, number: 9 };
/// assert!(new_master > old_master);
/// ```
// The derived order compares the fields in the order they are written, so `generation`
// must stay first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Token {
    /// Grows each time a new master takes over.
    pub generation: u64,
    /// The seniority number of the master in the line.
    pub number: u64,
}
