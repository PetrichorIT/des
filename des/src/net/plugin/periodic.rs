use std::panic::UnwindSafe;
use std::sync::atomic::Ordering::SeqCst;
use std::{any::Any, sync::atomic::AtomicU16, time::Duration};

use crate::net::message::TYP_HOOK_PERIODIC;
use crate::prelude::{schedule_in, Message};

use super::Plugin;

thread_local! {
    static PERIODIC_HOOK_ID: AtomicU16 = AtomicU16::new(0);
}

/// A plugin for running a certain calculation once in a periodic manner.
///
/// This hook can create a periodic F activity with a custom state S.
/// Upon creation the first wakup is scheduled with a period
/// provided by the caller.
///
/// Call deactivate to stop the periodic activity.
#[derive(Debug)]
pub struct PeriodicPlugin<S, F: Fn(&mut S)> {
    f: F,
    unique_id: u16,
    period: Duration,
    state: S,
    active: bool,
}

impl<S: Any, F: Fn(&mut S)> PeriodicPlugin<S, F> {
    /// Creates a new unqiue hook.
    pub fn new(f: F, period: Duration, state: S) -> Self {
        let this = Self {
            f,
            state,
            period,
            unique_id: PERIODIC_HOOK_ID.with(|v| v.fetch_add(1, SeqCst)),
            active: true,
        };

        log::trace!(
            "<PeriodicActivityHook> Created hook #{} with period {:?}",
            this.unique_id,
            this.period
        );
        this.send_wakeup();
        this
    }

    /// Removes all future wakeups.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    fn send_wakeup(&self) {
        if self.active {
            schedule_in(
                Message::new()
                    .typ(TYP_HOOK_PERIODIC)
                    .id(self.unique_id)
                    .content("PeriodicPlugin::WakupMessage")
                    .build(),
                self.period,
            )
        }
    }
}

impl<S: Any, F: Fn(&mut S)> Plugin for PeriodicPlugin<S, F>
where
    S: UnwindSafe,
    F: UnwindSafe,
{
    fn capture(&mut self, msg: Option<Message>) -> Option<Message> {
        let msg = msg?;
        if msg.header().typ == TYP_HOOK_PERIODIC && msg.header().id == self.unique_id {
            let f = &self.f;
            log::trace!("<PeriodicActivityHook> Actvitated Hook #{}", self.unique_id);
            f(&mut self.state);
            self.send_wakeup();
            None
        } else {
            Some(msg)
        }
    }

    fn defer(&mut self) {}
}
