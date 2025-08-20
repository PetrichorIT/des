#![cfg(feature = "net")]

use des::{
    net::{channel::DelayChannel, handlers::AsyncHandler},
    prelude::*,
};
use serial_test::serial;

#[derive(Default)]
struct DropChanModule {
    send: usize,
    received: usize,
}

impl Module for DropChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::default().with_content([0u8; 512]), "out");
        send(Message::default().with_content([1u8; 512]), "out");

        self.send += 2;
    }

    fn handle_message(&mut self, _msg: Message) {
        self.received += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_ne!(self.send, self.received);
        Ok(())
    }
}

#[test]
#[serial]
fn channel_dropping_message() {
    let mut rt = Sim::new(());
    rt.node("root", DropChanModule::default());

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = DatarateChannel::new(DatarateChannelMetrics {
        bitrate: 1000,
        latency: Duration::from_millis(100),
        jitter: Duration::ZERO,
        drop_behaviour: ChannelDropBehaviour::default(),
    });
    g_in.connect_with(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt.freeze());
    let _ = rt.run();
}

#[derive(Debug, Default)]
struct BufferChanModule {
    send: usize,
    received: usize,
}

impl Module for BufferChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::default().with_content([0u8; 512]), "out");
        send(Message::default().with_content([1u8; 512]), "out");
        send(Message::default().with_content([1u8; 512]), "out");

        self.send += 3;
    }

    fn handle_message(&mut self, _msg: Message) {
        self.received += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.send, 3);
        assert_eq!(self.received, 2);
        Ok(())
    }
}

#[test]
#[serial]
fn channel_buffering_message() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = Sim::new(());
    rt.node("root", BufferChanModule::default());

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = DatarateChannel::new(DatarateChannelMetrics {
        bitrate: 1000,
        latency: Duration::from_millis(100),
        jitter: Duration::ZERO,
        drop_behaviour: ChannelDropBehaviour::Queue(Some(600)),
    });
    g_in.connect_with(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt.freeze());
    let _ = rt.run();
}

struct SendMessageModule;
impl Module for SendMessageModule {
    fn at_sim_start(&mut self, _stage: usize) {
        schedule_in(Message::default().with_kind(10), Duration::from_secs(1));
    }

    fn handle_message(&mut self, msg: Message) {
        if msg.header().kind == 10 {
            send(Message::default().with_content("Hello world"), "out");
            let gate = current().gate("out", 0).unwrap();
            let ch = gate.channel().unwrap();
            assert!(ch.is_busy());
        }
    }
}

#[test]
#[serial]
fn channel_instant_busy() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = Sim::new(());
    rt.node("root", SendMessageModule);

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = DatarateChannel::new(DatarateChannelMetrics::new(
        1000,
        Duration::from_millis(100),
        Duration::ZERO,
        ChannelDropBehaviour::default(),
    ));

    g_in.connect_with(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt.freeze());
    let _ = rt.run();
}

struct LatencyOnly(usize);

impl Module for LatencyOnly {
    fn at_sim_start(&mut self, _stage: usize) {
        for _ in 0..10 {
            send(Message::default(), "out");
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        self.0 += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.0, 10);
        Ok(())
    }
}

#[test]
#[serial]
fn latency_only_channel() {
    let mut sim = Sim::new(());
    sim.node("alice", LatencyOnly(0));
    let gout = sim.gate("alice", "out");
    let gin = sim.gate("alice", "in");
    gout.connect_with(
        gin,
        Some(DatarateChannel::new(DatarateChannelMetrics::new(
            1000,
            Duration::from_secs(1),
            Duration::ZERO,
            ChannelDropBehaviour::Drop,
        ))),
    );

    let _ = Builder::seeded(123).build(sim.freeze()).run();
}

#[test]
#[serial]
fn simplex_shared_domain() {
    let mut sim = Sim::new(());

    sim.node(
        "receiver",
        AsyncHandler::new(|mut rx| async move {
            for _ in 0..25 {
                let msg = rx.recv().await.unwrap();
                let last = msg.last_gate.clone().unwrap();
                send(msg, last);
                tracing::info!("received message");
            }
        })
        .require_join(),
    );

    let switch = sim.gates("receiver", "mobile", 5);
    let to_receiver = ChannelRef::from(DatarateChannel::new(DatarateChannelMetrics::new(
        64 * 8,
        Duration::ZERO,
        Duration::ZERO,
        ChannelDropBehaviour::Queue(None),
    )));
    let to_sender = ChannelRef::from(DatarateChannel::new(DatarateChannelMetrics::new(
        64 * 8,
        Duration::ZERO,
        Duration::ZERO,
        ChannelDropBehaviour::Queue(None),
    )));

    for i in 0..5 {
        let key = format!("sender-{i}");
        sim.node(
            &key,
            AsyncHandler::new(|_| async move {
                for _ in 0..5 {
                    send(Message::default(), "mobile");
                }
            }),
        );
        let gate = sim.gate(key, "mobile");

        gate.clone().connect_with(
            switch[i].clone(),
            Some((to_receiver.clone(), to_sender.clone())),
        );
    }

    let rt = Builder::seeded(123).build(sim.freeze()).run().unwrap();

    assert_eq!(rt.2.event_count, 50 * 3);
    assert_eq!(rt.1, 26.0)
}

#[test]
#[serial]
fn duplex_shared_domain() {
    let mut sim = Sim::new(());

    sim.node(
        "receiver",
        AsyncHandler::new(|mut rx| async move {
            for _ in 0..25 {
                let msg = rx.recv().await.unwrap();
                let last = msg.last_gate.clone().unwrap();
                send(msg, last);
                tracing::info!("received message");
            }
        })
        .require_join(),
    );

    let switch = sim.gates("receiver", "mobile", 5);
    let chan = ChannelRef::from(DatarateChannel::new(DatarateChannelMetrics::new(
        64 * 8,
        Duration::ZERO,
        Duration::ZERO,
        ChannelDropBehaviour::Queue(None),
    )));

    for i in 0..5 {
        let key = format!("sender-{i}");
        sim.node(
            &key,
            AsyncHandler::new(|_| async move {
                for _ in 0..5 {
                    send(Message::default(), "mobile");
                }
            }),
        );
        let gate = sim.gate(key, "mobile");

        gate.clone()
            .connect_with(switch[i].clone(), Some(chan.clone()));
    }

    let rt = Builder::seeded(123).build(sim.freeze()).run().unwrap();

    assert_eq!(rt.2.event_count, 50 * 3);
    assert_eq!(rt.1, 50.0)
}

#[test]
#[serial]
fn channel_as_any() {
    let mut sim = Sim::new(());
    sim.node("alice", ());
    sim.node("bob", ());

    let g1 = sim.gate("alice", "port");
    let g2 = sim.gate("bob", "port");

    g1.clone()
        .connect_with(g2, Some(DelayChannel::new(Duration::from_millis(100))));
    let ch = g1.channel().unwrap();

    assert_eq!(
        ch.downcast_ref::<DelayChannel>().unwrap().delay,
        Duration::from_millis(100)
    );
}
