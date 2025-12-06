use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::net::module::ModuleContext;

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
        .await
    }
}
