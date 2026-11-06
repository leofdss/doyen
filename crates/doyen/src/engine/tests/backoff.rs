// SPDX-License-Identifier: MIT OR Apache-2.0
//! The delays between the retries (design.md, "Join, retries and reconnect").

use std::time::Duration;

use crate::engine::joining::Backoff;

fn within_jitter(delay: Duration, base_ms: u64) -> bool {
    let base = Duration::from_millis(base_ms);
    delay >= base.mul_f64(0.8) && delay <= base.mul_f64(1.2)
}

#[test]
fn the_delay_starts_at_100_ms_doubles_and_stops_at_the_renew_interval() {
    let mut backoff = Backoff::new(Duration::from_millis(2_000));
    let expected_ms = [100, 200, 400, 800, 1_600, 2_000, 2_000];

    for base_ms in expected_ms {
        let delay = backoff.next_delay();
        assert!(within_jitter(delay, base_ms), "{delay:?} for {base_ms} ms");
    }
}

#[test]
fn a_renew_interval_below_100_ms_is_the_first_delay() {
    let mut backoff = Backoff::new(Duration::from_millis(50));

    let delay = backoff.next_delay();

    assert!(within_jitter(delay, 50), "{delay:?}");
}
