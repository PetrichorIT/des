use std::{
    fmt::Debug,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use des_sync_utils::RwLock;
use futures::FutureExt;
use tokio::sync::oneshot;

use crate::{
    module::{ModuleContext, ModuleRef, SignalCode, current},
    runtime::{AsyncWakeupEvent, NetEvents},
    schedule_event,
    time::{SimTime, sleep},
};

use super::State;

/// A future that waits for a state to change
#[derive(Debug)]
struct WaitForStateChangeFuture {
    desired: State,
    handle: Arc<ModuleContext>,
}

impl Future for WaitForStateChangeFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.handle.state.get() == self.desired {
            Poll::Ready(())
        } else {
            self.handle
                .state_change_wakers
                .write()
                .push(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl ModuleContext {
    /// Waits till the simulation has been started
    pub async fn wait_for_start(self: Arc<Self>) {
        WaitForStateChangeFuture {
            desired: State::Running,
            handle: self,
        }
        .await;
    }
}

pub struct SyncPoint {
    inner: Arc<SyncPointInner>,
    rx: Option<(oneshot::Receiver<()>, ModuleRef)>,
}

struct SyncPointInner {
    key: Option<SignalCode>,
    points: RwLock<Vec<(oneshot::Sender<()>, ModuleRef)>>,
    autotrigger: RwLock<bool>,
}

impl SyncPoint {
    pub fn new() -> Self {
        Self::new_with_key(None)
    }

    pub fn new_with_key(key: Option<SignalCode>) -> Self {
        Self {
            inner: Arc::new(SyncPointInner {
                key,
                points: RwLock::new(Vec::new()),
                autotrigger: RwLock::new(false),
            }),
            rx: None,
        }
    }

    pub fn key(&self) -> Option<SignalCode> {
        self.inner.key.clone()
    }

    pub fn trigger(&self) {
        let mut lock = self.inner.points.write();
        for (tx, module) in lock.drain(..) {
            tx.send(()).ok();
            schedule_event(
                NetEvents::AsyncWakeupEvent(AsyncWakeupEvent { module }),
                SimTime::now(),
            );
        }
    }

    pub fn set_autotrigger(&self) {
        *self.inner.autotrigger.write() = true
    }
}

impl Debug for SyncPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncPoint")
            .field("key", &self.inner.key)
            .field("scheduled", &self.inner.points.read().len())
            .finish()
    }
}

impl Clone for SyncPoint {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            rx: None,
        }
    }
}

impl Future for SyncPoint {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let existing = self.as_mut().rx.take();
        let existing = existing.and_then(|tup| tup.1.is_currently_active().then_some(tup));
        let rx_and_mod = existing.unwrap_or_else(|| {
            let mut points = self.inner.points.write();

            let (tx, rx) = oneshot::channel();
            if points.is_empty() && *self.inner.autotrigger.read() {
                let inner = self.clone();
                tokio::spawn(async move {
                    sleep(Duration::from_secs(1)).await;
                    inner.trigger();
                });
            }
            points.push((tx, current().me()));
            (rx, current().me())
        });

        self.rx = Some(rx_and_mod);
        self.as_mut()
            .rx
            .as_mut()
            .unwrap()
            .0
            .poll_unpin(cx)
            .map(|_| ())
    }
}
