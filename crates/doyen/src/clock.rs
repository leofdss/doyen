// SPDX-License-Identifier: MIT OR Apache-2.0
//! The clock that counts the time of a suspend.

use std::time::Duration;

// Supported targets (spec rust-api, "Supported platforms"): Linux on x86_64 and aarch64, and
// macOS on aarch64. Each has a clock that counts the time of a suspend. Windows has one
// too (`QueryInterruptTime`) and may come later.
#[cfg(not(any(
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ),
    all(target_os = "macos", target_arch = "aarch64"),
)))]
compile_error!(
    "doyen supports only Linux (x86_64 and aarch64) and macOS (aarch64): \
     it needs a monotonic clock that counts the time of a suspend"
);

/// The clock that counts suspend. Linux has `CLOCK_BOOTTIME` for it. On macOS,
/// `CLOCK_MONOTONIC` is the one that counts sleep (`CLOCK_UPTIME_RAW`, which Rust's
/// `Instant` uses there, does not).
#[cfg(target_os = "linux")]
const SUSPEND_AWARE_CLOCK: rustix::time::ClockId = rustix::time::ClockId::Boottime;
#[cfg(target_os = "macos")]
const SUSPEND_AWARE_CLOCK: rustix::time::ClockId = rustix::time::ClockId::Monotonic;

#[cfg(test)]
mod tests;

/// A monotonic time that keeps counting while the host sleeps.
///
/// The lease uses it: after a resume the lease must be over, even if the process did not
/// run. `Instant` of the standard library does not count sleep on macOS.
pub(crate) trait Clock: Send + Sync + 'static {
    /// The time since an arbitrary start. Only the difference of two readings means
    /// something.
    fn now(&self) -> Duration;
}

/// The real clock of the host.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        let reading = rustix::time::clock_gettime(SUSPEND_AWARE_CLOCK);
        // A reading is never negative; 0 is only a fallback that cannot happen.
        let seconds = u64::try_from(reading.tv_sec).unwrap_or(0);
        let nanoseconds = u32::try_from(reading.tv_nsec).unwrap_or(0);
        Duration::new(seconds, nanoseconds)
    }
}

/// A clock for tests, on the time of Tokio: with `tokio::time::pause()` it moves only when
/// the test advances it, together with the timers.
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct FakeClock {
    start: tokio::time::Instant,
}

#[cfg(test)]
impl FakeClock {
    pub(crate) fn new() -> FakeClock {
        FakeClock {
            start: tokio::time::Instant::now(),
        }
    }
}

#[cfg(test)]
impl Clock for FakeClock {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }
}
