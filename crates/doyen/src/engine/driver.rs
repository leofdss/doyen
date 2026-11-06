// SPDX-License-Identifier: MIT OR Apache-2.0
//! The driver: one Tokio task per joined object. It renews the place at a fixed rate and
//! after each wake-up, sends the notification when the lease ends, and joins again when the
//! place is lost. It stops on `leave` or when the last handle is dropped.

use std::future::Future;
use std::sync::Arc;

use tokio::time::{Instant, MissedTickBehavior};

use super::joining::{self, Backoff, Connection, RenewOutcome};
use super::state::Shared;
use super::stop::StopSignals;
use super::wake_ups::WakeUps;

pub(super) struct Driver {
    shared: Arc<Shared>,
    connection: Connection,
    signals: StopSignals,
    /// The fixed-rate renews, counted from the join (election-core, "Renews at a fixed
    /// rate").
    schedule: tokio::time::Interval,
    wake_ups: WakeUps,
}

enum Event {
    Stop,
    Renew,
    LeaseEnded,
}

impl Driver {
    /// Start the driver on the current Tokio runtime. It keeps only `Shared`, never a
    /// handle, so it cannot keep the object alive.
    pub(super) fn start(
        shared: Arc<Shared>,
        connection: Connection,
        signals: StopSignals,
        mut wake_ups: WakeUps,
    ) {
        wake_ups.skip_waiting_wake_ups();
        let interval = connection.renew_interval();
        let mut schedule = tokio::time::interval_at(Instant::now() + interval, interval);
        // A tick missed after a suspend or a slow call does not send a burst of renews.
        schedule.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let driver = Driver {
            shared,
            connection,
            signals,
            schedule,
            wake_ups,
        };
        // A panic in the driver only ends this task: nothing renews, and the lease ends on
        // its own (election-core, "Renew task panics").
        tokio::spawn(driver.run());
    }

    async fn run(mut self) {
        loop {
            let keep_going = match self.next_event().await {
                Event::Stop => false,
                Event::Renew => self.renew().await,
                Event::LeaseEnded => {
                    self.shared.publish_status();
                    true
                }
            };
            if !keep_going {
                return;
            }
        }
    }

    async fn next_event(&mut self) -> Event {
        let lease_deadline = self.shared.lease_deadline();
        tokio::select! {
            biased;
            () = self.signals.stopped() => Event::Stop,
            _ = self.schedule.tick() => Event::Renew,
            // An extra renew: it does not move the schedule.
            () = self.wake_ups.next() => Event::Renew,
            () = sleep_until_if_set(lease_deadline) => Event::LeaseEnded,
        }
    }

    /// One renew. `false` when the driver must stop.
    async fn renew(&mut self) -> bool {
        let Some(membership) = self.shared.membership() else {
            return true;
        };
        // A renew from an app that cannot work would keep a dead master in its place. With
        // no renew, the lease ends and the other members take over as after a crash.
        if !self.shared.is_live() {
            tracing::debug!(
                target: "doyen",
                group = self.connection.group(),
                "the liveness check failed, so the renew is skipped"
            );
            return true;
        }
        let renewing = joining::renew(&self.shared, &self.connection, membership);
        let outcome = watch_the_lease(&self.shared, &mut self.signals, renewing).await;
        match outcome {
            None => false,
            Some(RenewOutcome::Renewed(place)) => self.shared.hold(place),
            Some(RenewOutcome::Failed) => true,
            Some(RenewOutcome::PlaceGone) => self.join_again().await,
        }
    }

    /// The place is gone: join again with a new membership until there is a place. After
    /// the first place the core never gives up, so a refusal is retried too
    /// (election-core, "After join the core never gives up").
    async fn join_again(&mut self) -> bool {
        self.shared.place_lost();
        let mut backoff = Backoff::new(self.connection.renew_interval());
        loop {
            let joining = joining::join_until_placed(&self.shared, &self.connection, &mut backoff);
            match self.signals.unless_stopped(joining).await {
                None => return false,
                // `false` when `leave` came first; the new place then expires on its own.
                Some(Ok(place)) => return self.shared.hold(place),
                Some(Err(refusal)) => self.shared.call_failed("join", &refusal.to_string()),
            }
            let pause = tokio::time::sleep(backoff.next_delay());
            if self.signals.unless_stopped(pause).await.is_none() {
                return false;
            }
        }
    }
}

/// Run `work` until it ends or a stop signal comes, and send the notification at the lease
/// end if it comes meanwhile (a stuck call must not hide the end of the lease).
async fn watch_the_lease<T>(
    shared: &Shared,
    signals: &mut StopSignals,
    work: impl Future<Output = T>,
) -> Option<T> {
    let lease_deadline = shared.lease_deadline();
    let work = signals.unless_stopped(work);
    tokio::pin!(work);
    tokio::select! {
        output = &mut work => return output,
        () = sleep_until_if_set(lease_deadline) => shared.publish_status(),
    }
    work.await
}

async fn sleep_until_if_set(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}
