use std::{any::Any, time::Duration};

use des::{
    net::{
        Sim,
        channel::{SendContext, SendError},
        gate::Connection,
        handlers::HandlerFn,
        internals::{ChannelUnbusyNotif, MessageExitingConnection, NetEvents},
    },
    prelude::{Channel, ChannelRef, GateRef, Message, Module, current, send},
    runtime::Builder,
    time::{SimTime, interval, sleep_until},
};

struct Sender;

impl Module for Sender {
    fn at_sim_start(&mut self, _stage: usize) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(200));
            interval.set_missed_tick_behavior(des::time::MissedTickBehavior::Delay);
            for i in 0..10 {
                interval.tick().await;
                send_radio(
                    Message::default().with_id(i),
                    current().gate("port", 0).unwrap(),
                )
                .await;
            }
        });
    }
}

struct TimeDividedRadioChannel {
    prop_delay: Duration,
    datarate: usize,

    transmission_finish_time: SimTime,
    period: Duration,
    slots: Vec<GateRef>,
    state: (SimTime, usize),
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
            state: (SimTime::ZERO, 0),
        }
    }

    fn send_window_for(&self, gate: GateRef) -> SimTime {
        assert!(!self.state.0.is_zero(), "Channel not yet initalized");
        let pos = self
            .slots
            .iter()
            .position(|slot| slot == &gate)
            .expect("gate not registered");

        if pos == self.state.1 {
            self.transmission_finish_time
        } else {
            self.slot_for(gate)
        }
    }

    fn slot_for(&self, gate: GateRef) -> SimTime {
        assert!(!self.state.0.is_zero(), "Channel not yet initalized");
        let pos = self
            .slots
            .iter()
            .position(|slot| slot == &gate)
            .expect("gate not registered");

        let cur = self.state.1;

        let delta = if pos > cur {
            pos - cur
        } else {
            (pos + self.slots.len()) - cur
        };

        let deadline = self.state.0 + self.period * (delta - 1) as u32;
        assert!(
            deadline > SimTime::now(),
            "{deadline} is not valid (now = {})",
            SimTime::now()
        );
        deadline
    }

    fn next_slot<'ctx>(&mut self, ctx: &mut SendContext<'ctx>) {
        let n = self.slots.len();
        let i = (self.state.1 + 1) % n;
        let deadline = SimTime::now() + self.period;
        self.state = (deadline, i);

        ctx.sink.add(
            NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif {
                channel: ctx.handle.clone(),
                info: Box::new(NotifType::NextSlot(deadline)),
            }),
            deadline,
        );
    }

    fn is_current_slot(&mut self, gate: GateRef, ctx: &mut SendContext<'_>) -> bool {
        let is_match = self.slots[self.state.1] == gate;
        if is_match {
            return true;
        }

        let new_slot_beginning_but_not_yet_notified = self.state.0 == SimTime::now();
        if new_slot_beginning_but_not_yet_notified {
            self.next_slot(ctx);
            return true;
        }

        false
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

        if self.state.0.is_zero() {
            self.next_slot(&mut ctx);
        }

        if self.is_current_slot(src, &mut ctx) {
            if self.transmission_finish_time <= SimTime::now() {
                let transmit_time = (message.length() * 8) as f64 / self.datarate as f64;
                self.transmission_finish_time = SimTime::now() + transmit_time;

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
                    SimTime::now() + self.prop_delay,
                );

                Ok(())
            } else {
                Err(SendError {
                    msg: message,
                    reason: "channel busy".into(),
                })
            }
        } else {
            return Err(SendError {
                msg: message,
                reason: "not your sending slot".into(),
            });
        }
    }

    fn unbusy_notify<'ctx>(&mut self, info: Box<dyn Any + Send>, mut ctx: SendContext<'ctx>) {
        let info = info.downcast_ref::<NotifType>().unwrap();
        match info {
            NotifType::NextSlot(time) if *time == self.state.0 => self.next_slot(&mut ctx),
            NotifType::NextSlot(_) => (), // slot moveover was allready handled by other activation
            NotifType::TransmittorReady => {}
        }
    }
}

async fn send_radio(message: impl Into<Message>, gate: GateRef) {
    let mut msg = message.into();
    loop {
        match send(msg, gate.clone()) {
            Ok(()) => break,
            Err(e) => {
                msg = e.msg;
                let chan = gate
                    .channel()
                    .unwrap()
                    .downcast_ref::<TimeDividedRadioChannel, _>(|chan| {
                        chan.send_window_for(gate.clone())
                    })
                    .unwrap(); // TODO remove

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
        HandlerFn::new(|msg| tracing::info!("#{} from {}", msg.id, msg.header.sender_module_id)),
    );

    let chan =
        TimeDividedRadioChannel::new(Duration::from_millis(0), 6_400, Duration::from_millis(500));
    let shared = ChannelRef::from(chan);

    for i in 0..3 {
        sim.node(format!("client-{i}"), Sender);
        let g = sim.gate(&format!("client-{i}"), "port");
        let gt = sim.gate("tower", &format!("port-{i}"));
        g.connect_with(gt, Some(shared.clone()));
    }

    let _ = Builder::seeded(123)
        .max_time(10.0.into())
        .build(sim.freeze())
        .run()
        .unwrap();
}
