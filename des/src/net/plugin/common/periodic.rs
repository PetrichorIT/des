use crate::{
    net::{message::TYP_PLUGIN_PERIODIC, plugin::Plugin},
    prelude::{schedule_in, Message},
    time::{Duration, SimTime},
};

/// A periodic plugin.
#[derive(Debug)]
pub struct PeriodicPlugin<S, F: Fn(&mut S)> {
    f: F,
    state: S,

    period: Duration,
    active: bool,
    next_wakeup: SimTime,
}

impl<S, F: Fn(&mut S)> PeriodicPlugin<S, F> {
    /// Creates a new unqiue plugin.
    pub fn new(f: F, period: Duration, state: S) -> Self {
        let mut this = Self {
            f,
            state,

            period,
            active: true,
            next_wakeup: SimTime::now(),
        };

        this.send_wakeup();
        this
    }

    /// Removes all future wakeups.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    fn send_wakeup(&mut self) {
        if self.active {
            schedule_in(
                Message::new()
                    .typ(TYP_PLUGIN_PERIODIC)
                    .content("PeriodicPlugin::WakupMessage")
                    .build(),
                self.period,
            );
            self.next_wakeup = SimTime::now() + self.period;
        }
    }
}

impl<S, F: Fn(&mut S)> Plugin for PeriodicPlugin<S, F>
where
    S: 'static,
    F: 'static,
{
    fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
        if msg.header().typ == TYP_PLUGIN_PERIODIC && SimTime::now() == self.next_wakeup {
            let f = &self.f;
            f(&mut self.state);
            self.send_wakeup();
            None
        } else {
            Some(msg)
        }
    }
}
