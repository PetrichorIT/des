use std::{any::Any, time::Duration};

use des::{
    Sim,
    channel::{SendContext, SendError},
    gate::{Connection, IntoGate, IntoModuleGate},
    prelude::{Channel, ChannelRef, GateRef, Message, Module, current, send},
    runtime::{ChannelUnbusyNotif, MessageExitingConnection, NetEvents, handlers::HandlerFn},
    time::{SimTime, interval, sleep_until},
};

// Create a sender module that sends messages in 200ms intervals,
// blocking when the channel cannot currently serve the module.
struct Sender;

impl Module for Sender {
    fn at_sim_start(&mut self, _stage: usize) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(200));
            interval.set_missed_tick_behavior(des::time::MissedTickBehavior::Delay);
            for i in 0..10 {
                interval.tick().await;
                send_radio(Message::default().with_id(i), "port").await;
            }
        });
    }
}

/// A time-divided radio channel that requires registration of all transmitters.
struct TimeDividedRadioChannel {
    // Internally the radio domain still acts as a "DatarateChannel",
    // thus a propagation delay and bitrate is required.
    prop_delay: Duration,
    datarate: usize,
    // If the transmitter is currently active, store the time it will be idle again.
    transmission_finish_time: SimTime,

    // To manage time division multiplexing, keep track of all registered transmitters,
    // the current time slot (and it deadline) and the configurated length of each time slot.
    period: Duration,
    slots: Vec<GateRef>,

    current_active_slot: usize,
    current_slot_end_time: SimTime,
}

#[derive(Debug)]
enum NotifType {
    NextSlot(SimTime),
    TransmittorReady,
}

impl TimeDividedRadioChannel {
    fn new(prop_delay: Duration, datarate: usize, period: Duration) -> Self {
        Self {
            prop_delay,
            datarate,
            period,

            transmission_finish_time: SimTime::ZERO,
            slots: Vec::default(),

            current_active_slot: 0,
            current_slot_end_time: SimTime::ZERO,
        }
    }

    /// Checks whether a given sender can send a given message over the medium.
    /// Will either succeed with Ok(transmit_time) or Err(wait_until)
    ///
    /// There are three possible reasons a send request might be rejected:
    /// - The senders window is not currently active (-> wait until the senders window)
    /// - The senders window is active, but the transmittor is currently busy sending the previous message (-> wait until the transmittor is free)
    /// - The remaining space in the active window is not sufficient to transmit the message (-> wait until the next window)
    fn should_wait_until(
        &self,
        gate: GateRef,
        msg: &Message,
        now: SimTime,
    ) -> Result<Duration, SimTime> {
        assert_ne!(now, self.current_slot_end_time, "we are in a healthy slot");

        if gate != self.slots[self.current_active_slot] {
            return Err(self.next_window_for(gate));
        }

        let msg_bitlen = msg.length() * 8;
        let msg_transmit_time = Duration::from_secs_f64(msg_bitlen as f64 / self.datarate as f64);
        assert!(
            msg_transmit_time <= self.period,
            "This message is simply impossible to transmit"
        );

        if self.transmission_finish_time > now {
            return Err(self.transmission_finish_time);
        }

        let remaining_time_in_slot = self.current_slot_end_time - now;
        if remaining_time_in_slot < msg_transmit_time {
            return Err(self.next_window_for(gate));
        }

        Ok(msg_transmit_time)
    }

    fn next_window_for(&self, gate: GateRef) -> SimTime {
        let pos = self
            .slots
            .iter()
            .position(|slot| slot == &gate)
            .expect("gate not registered");

        // Calculates the expected next window. If another node is registred in the mean time,
        // this window may be to early, but never to late: thus another call will solve the issue.
        let delta = if pos > self.current_active_slot {
            pos - self.current_active_slot
        } else {
            (pos + self.slots.len()) - self.current_active_slot
        };

        let deadline = self.current_slot_end_time + self.period * (delta - 1) as u32;
        deadline
    }

    /// Advances the channel to the next slot, by incrementing the current slot index
    /// and rescheduling the notification.
    fn advance_window<'ctx>(&mut self, ctx: &mut SendContext<'ctx>, now: SimTime) {
        let n = self.slots.len();
        self.current_active_slot = (self.current_active_slot + 1) % n;
        self.current_slot_end_time = now + self.period;

        ctx.sink.add(
            NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif {
                channel: ctx.handle.clone(),
                info: Box::new(NotifType::NextSlot(self.current_slot_end_time)),
            }),
            self.current_slot_end_time,
        );
    }
}

impl Channel for TimeDividedRadioChannel {
    fn transmission_finish_time(&self) -> Option<SimTime> {
        Some(self.transmission_finish_time)
    }

    fn register(&mut self, endpoint: GateRef) {
        assert!(self.slots.len() < 7);
        self.slots.push(endpoint);
    }

    fn send<'ctx>(
        &mut self,
        src: GateRef,
        message: Message,
        via: Connection,
        mut ctx: SendContext<'ctx>,
    ) -> Result<(), SendError> {
        assert!(self.slots.contains(&src), "invalid call");

        let now = SimTime::now();

        // Initalize the self-notifications if not yet done or
        // advance the slot if it just ended, but notif was not yet processed
        if self.current_slot_end_time.is_zero() || self.current_slot_end_time == now {
            self.advance_window(&mut ctx, now);
        }

        match self.should_wait_until(src, &message, now) {
            Err(wait) => Err(SendError {
                msg: message,
                reason: format!("wait for {}", wait),
            }),
            Ok(msg_transmit) => {
                self.transmission_finish_time = now + msg_transmit;

                ctx.sink.add(
                    NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif {
                        channel: ctx.handle.clone(),
                        info: Box::new(NotifType::TransmittorReady),
                    }),
                    self.transmission_finish_time,
                );

                ctx.sink.add(
                    NetEvents::MessageExitingConnection(MessageExitingConnection {
                        con: via,
                        msg: message,
                    }),
                    now + self.prop_delay, // TODO: + msg_transmit
                );

                Ok(())
            }
        }
    }

    fn unbusy_notify<'ctx>(&mut self, info: Box<dyn Any + Send>, mut ctx: SendContext<'ctx>) {
        let info = info.downcast_ref::<NotifType>().unwrap();
        match info {
            NotifType::NextSlot(time) if *time == self.current_slot_end_time => {
                self.advance_window(&mut ctx, SimTime::now())
            }
            NotifType::NextSlot(_) => (), // slot moveover was allready handled by other activation
            NotifType::TransmittorReady => {} // do somthing once the sender is active again
        }
    }
}

/// Since send operations can fail, due to TDM a custom send function is needed to preserve
/// the `send`-like API.
///
/// This function trys to send the message and if it fails, accesses the channel to get the next
/// send window according to the channels internals, sleeping until then.
async fn send_radio(message: impl Into<Message>, gate: impl IntoModuleGate) {
    let mut msg = message.into();
    let gate = gate.as_gate(&current()).expect("failed to get gate");
    loop {
        match send(msg, &gate) {
            Ok(()) => break,
            Err(e) => {
                msg = e.msg;
                let chan = gate
                    .channel()
                    .expect("we know there is a channel")
                    .downcast_ref::<TimeDividedRadioChannel, _>(|chan| {
                        chan.should_wait_until(gate.clone(), &msg, SimTime::now())
                            .expect_err("we expect to wait")
                    })
                    .expect("and we know its of type TimteDivivdedRadioChannel");

                sleep_until(chan).await;
            }
        }
    }
}

fn main() {
    des::tracing::init();

    let mut sim = Sim::new(());
    sim.node(
        "tower",
        HandlerFn::new(|msg| tracing::info!("#{} from", msg.id)),
    );

    // Create a channel, that will be shared by casting it to a ChannelRef
    let chan =
        TimeDividedRadioChannel::new(Duration::from_millis(0), 6_400, Duration::from_millis(500));
    let shared = ChannelRef::from(chan);

    // Reuse the channel for all gate-connections
    for i in 0..3 {
        sim.node(format!("client-{i}"), Sender);
        let g = sim.gate(&format!("client-{i}"), "port");
        let gt = sim.gate("tower", &format!("port-{i}"));
        g.connect_with(gt, Some(shared.clone()));
    }

    let _ = sim
        .seeded(123)
        .max_time(10.0.into())
        .build()
        .run()
        .assert_no_err();
}
