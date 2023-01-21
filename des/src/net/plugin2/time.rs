use std::panic::UnwindSafe;

use tokio::{
    sim::SimContext,
    time::{SimTime, TimeContext},
};

use super::Plugin;
use crate::{
    net::message::{Message, TYP_WAKEUP},
    prelude::schedule_at,
};

/// Tokio-Intergration
#[derive(Debug)]
pub struct TokioTimePlugin {
    time: Option<TimeContext>,
    prev: Option<TimeContext>,
    next_wakeup: SimTime,
}

impl TokioTimePlugin {
    /// Creates a new instance of self
    #[must_use]
    pub fn new(ident: String) -> Self {
        Self {
            time: Some(TimeContext::new(ident)),
            prev: None,
            next_wakeup: SimTime::MAX,
        }
    }
}

// # SAFTEY:
// Once paniced the stored prev varients are no longer used.
impl UnwindSafe for TokioTimePlugin {}

impl Plugin for TokioTimePlugin {
    fn event_start(&mut self) {
        log::debug!("tokio::time::event_start");
        // (0) Swap in time context
        let mut time = self.time.take().expect("Plugin lost its time context");
        self.prev = SimContext::with_current(|ctx| {
            if let Some(tctx) = ctx.time.as_mut() {
                tctx.swap(&mut time);
                Some(time)
            } else {
                ctx.time = Some(time);
                Some(TimeContext::new("PLUGINREPLACE".to_string()))
            }
        });

        // (1) Handle current time events
        SimContext::with_current(|ctx| {
            if let Some(ctx) = ctx.time.as_mut() {
                ctx.process_now();
            }
        });
    }

    fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
        log::debug!("tokio::time::capture_incoming");
        if msg.header().typ == TYP_WAKEUP {
            self.next_wakeup = SimTime::MAX;
            None
        } else {
            Some(msg)
        }
    }

    fn capture_outgoing(&mut self, msg: Message) -> Option<Message> {
        log::debug!("tokio::time::capture_outgoing");
        Some(msg)
    }

    fn event_end(&mut self) {
        log::debug!("tokio::time::event_end");
        SimContext::with_current(|ctx| {
            let Some(ctx) = ctx.time.as_mut() else { return };
            let Some(next_time) = ctx.next_time_poll() else { return };

            if next_time <= self.next_wakeup {
                self.next_wakeup = next_time;
                schedule_at(Message::new().typ(TYP_WAKEUP).build(), next_time);
            }
        });

        // (0) Swap in time context
        let was_taken = self.time.is_none();
        if was_taken {
            let mut time = self.prev.take().unwrap();
            SimContext::with_current(|ctx| {
                if let Some(tctx) = ctx.time.as_mut() {
                    tctx.swap(&mut time);
                } else {
                    unreachable!()
                }
            });

            self.time = Some(time);
            assert!(self.time.is_some());
        }
    }
}
