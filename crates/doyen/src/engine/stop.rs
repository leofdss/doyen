// SPDX-License-Identifier: MIT OR Apache-2.0
//! The signals that stop the work of an object: `leave`, and the drop of the last handle.

use std::future::Future;

use tokio::sync::{oneshot, watch};

pub(super) struct StopSignals {
    /// `true` once `leave` started.
    leaving: watch::Receiver<bool>,
    /// Ends when the last handle of the object is dropped. Only the driver has it: a
    /// pending `join` holds a handle, so the handle cannot drop under it.
    last_handle_dropped: Option<oneshot::Receiver<()>>,
}

impl StopSignals {
    /// For a pending `join`: only `leave` stops it.
    pub(super) fn on_leave(leaving: watch::Receiver<bool>) -> StopSignals {
        StopSignals {
            leaving,
            last_handle_dropped: None,
        }
    }

    /// For the driver: `leave`, or the drop of the last handle.
    pub(super) fn on_leave_or_drop(
        leaving: watch::Receiver<bool>,
        last_handle_dropped: oneshot::Receiver<()>,
    ) -> StopSignals {
        StopSignals {
            leaving,
            last_handle_dropped: Some(last_handle_dropped),
        }
    }

    /// Wait until a signal comes. Safe to cancel.
    pub(super) async fn stopped(&mut self) {
        // An error means the sender is gone, so nothing can need this object any more.
        let leave_called = async {
            let _ = self.leaving.wait_for(|leaving| *leaving).await;
        };
        match &mut self.last_handle_dropped {
            Some(last_handle_dropped) => {
                tokio::select! {
                    () = leave_called => {}
                    _ = last_handle_dropped => {}
                }
            }
            None => leave_called.await,
        }
    }

    /// Run `work` until it ends or a signal comes. `None` means "stop now": the work was
    /// dropped (every adapter call is safe to cancel).
    pub(super) async fn unless_stopped<T>(&mut self, work: impl Future<Output = T>) -> Option<T> {
        tokio::select! {
            biased;
            () = self.stopped() => None,
            output = work => Some(output),
        }
    }
}
