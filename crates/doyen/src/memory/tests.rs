// SPDX-License-Identifier: MIT OR Apache-2.0
//! The conformance suite on the in-memory adapter, and the tests of its own switches.

use super::*;

/// The in-memory database runs on the fake clock: Tokio time is paused, and a wait moves
/// it at once.
async fn start_memory_database() -> Option<MemoryDatabase> {
    tokio::time::pause();
    Some(MemoryDatabase::new())
}

crate::conformance::conformance_tests!(start_memory_database);

#[tokio::test]
async fn the_switch_ends_the_watch_streams_of_the_group() {
    let database = start_memory_database().await.expect("always runs");
    let adapter = database.new_adapter().await;
    let mut wake_ups = adapter
        .watch("group")
        .await
        .expect("watch must work")
        .expect("the in-memory database has push");

    database.end_watch_streams("group");

    assert_eq!(wake_ups.next().await, None);
}
