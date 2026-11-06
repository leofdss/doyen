// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tests of the real clock and the fake clock.

use std::time::Duration;

use super::*;

#[test]
fn the_real_clock_measures_a_real_sleep() {
    let clock = SystemClock;
    let before = clock.now();

    std::thread::sleep(Duration::from_secs(2));

    let elapsed = clock.now() - before;
    assert!(elapsed >= Duration::from_secs(2), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(4), "{elapsed:?}");
}

#[test]
fn the_real_clock_never_goes_back() {
    let clock = SystemClock;
    let mut last = clock.now();

    for _ in 0..1000 {
        let now = clock.now();
        assert!(now >= last);
        last = now;
    }
}

#[tokio::test(start_paused = true)]
async fn the_fake_clock_moves_only_when_the_test_advances_time() {
    let clock = FakeClock::new();
    let start = clock.now();

    assert_eq!(clock.now(), start);

    tokio::time::advance(Duration::from_secs(5)).await;

    assert_eq!(clock.now() - start, Duration::from_secs(5));
}

#[tokio::test(start_paused = true)]
async fn the_fake_clock_moves_with_the_timers() {
    let clock = FakeClock::new();
    let start = clock.now();

    tokio::time::sleep(Duration::from_secs(9)).await;

    assert_eq!(clock.now() - start, Duration::from_secs(9));
}
