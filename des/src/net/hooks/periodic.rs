use super::Hook;
use crate::{
    net::{message::TYP_HOOK_PERIODIC, Message},
    prelude::schedule_in,
};
use std::{
    any::Any,
    sync::atomic::{AtomicU16, Ordering::SeqCst},
    time::Duration,
};

static PERIODIC_HOOK_ID: AtomicU16 = AtomicU16::new(0);

/// A hook for running a certain calculation once in a periodic manner.
#[derive(Debug)]
pub struct PeriodicHook<S, F: Fn(&mut S)> {
    f: F,
    unique_id: u16,
    period: Duration,
    state: S,
}

impl<S: Any, F: Fn(&mut S)> PeriodicHook<S, F> {
    /// Creates a new unqiue hook.
    pub fn new(f: F, period: Duration, state: S) -> Self {
        let this = Self {
            f,
            state,
            period,
            unique_id: PERIODIC_HOOK_ID.fetch_add(1, SeqCst),
        };

        this.send_wakeup();
        this
    }

    fn send_wakeup(&self) {
        schedule_in(
            Message::new()
                .typ(TYP_HOOK_PERIODIC)
                .id(self.unique_id)
                .build(),
            self.period,
        )
    }
}

impl<S: Any, F: Fn(&mut S)> Hook for PeriodicHook<S, F> {
    fn state(&self) -> &dyn std::any::Any {
        &self.state
    }

    fn handle_message(&mut self, msg: Message) -> Result<(), Message> {
        if msg.header().typ == TYP_HOOK_PERIODIC && msg.header().id == self.unique_id {
            let f = &self.f;
            f(&mut self.state);
            self.send_wakeup();
            Ok(())
        } else {
            Err(msg)
        }
    }
}
