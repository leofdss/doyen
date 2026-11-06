// SPDX-License-Identifier: MIT OR Apache-2.0
//! The wake-ups of the adapter's `watch` stream, opened again when it ends or fails
//! (election-core, "Wake-ups come back after an error").

use std::sync::Arc;

use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;
use futures_util::{FutureExt, StreamExt};
use tokio::time::Instant;

use super::joining::{Backoff, Connection, call_with_timeout};
use super::state::Shared;
use crate::backend;

type WakeUpStream = BoxStream<'static, ()>;

/// Where the watch stream is. The renew timer runs in every state.
enum WatchState {
    Opening(BoxFuture<'static, backend::Result<Option<WakeUpStream>>>),
    Open(WakeUpStream),
    /// The stream ended or could not open; it opens again at this time.
    WaitingToReopen(Instant),
    /// The database has no push: the timer alone.
    NoPush,
}

pub(super) struct WakeUps {
    state: WatchState,
    backoff: Backoff,
    shared: Arc<Shared>,
    connection: Connection,
}

impl WakeUps {
    /// Open the watch stream, with one call timeout. When it fails, the driver opens it
    /// again later.
    pub(super) async fn open(shared: Arc<Shared>, connection: Connection) -> WakeUps {
        let mut wake_ups = WakeUps {
            state: WatchState::NoPush,
            backoff: Backoff::new(connection.renew_interval()),
            shared,
            connection,
        };
        let opened = open_stream(wake_ups.connection.clone()).await;
        wake_ups.state = wake_ups.state_after_open(opened);
        wake_ups
    }

    /// Drop the wake-ups that are waiting now. At the start of the driver they came from my
    /// own join (or an attempt of it), or from a change that the join answer already showed:
    /// a renew for them would only read the same line.
    pub(super) fn skip_waiting_wake_ups(&mut self) {
        if let WatchState::Open(stream) = &mut self.state {
            while let Some(Some(())) = stream.next().now_or_never() {}
        }
    }

    /// Wait for the next wake-up, and open the stream again when it ends meanwhile. Safe to
    /// cancel: the state changes only after an `await` finished.
    pub(super) async fn next(&mut self) {
        loop {
            match &mut self.state {
                WatchState::Opening(opening) => {
                    let opened = opening.await;
                    self.state = self.state_after_open(opened);
                }
                WatchState::Open(stream) => match stream.next().await {
                    Some(()) => return,
                    None => self.state = self.reopen_later(),
                },
                WatchState::WaitingToReopen(reopen_at) => {
                    tokio::time::sleep_until(*reopen_at).await;
                    self.state = WatchState::Opening(open_stream(self.connection.clone()));
                }
                WatchState::NoPush => std::future::pending::<()>().await,
            }
        }
    }

    fn state_after_open(&mut self, opened: backend::Result<Option<WakeUpStream>>) -> WatchState {
        match opened {
            Ok(Some(stream)) => {
                self.backoff = Backoff::new(self.connection.renew_interval());
                WatchState::Open(stream)
            }
            Ok(None) => WatchState::NoPush,
            Err(error) => {
                self.shared.call_failed("watch", &error.to_string());
                self.reopen_later()
            }
        }
    }

    fn reopen_later(&mut self) -> WatchState {
        WatchState::WaitingToReopen(Instant::now() + self.backoff.next_delay())
    }
}

fn open_stream(
    connection: Connection,
) -> BoxFuture<'static, backend::Result<Option<WakeUpStream>>> {
    Box::pin(async move {
        let watching = connection.backend.watch(connection.group());
        call_with_timeout(&connection, watching).await
    })
}
