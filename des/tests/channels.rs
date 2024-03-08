#![cfg(feature = "net")]
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use des::{
    net::channel::{ChannelDropBehaviour, ChannelProbe},
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
        send(Message::new().content([0u8; 512]).build(), "out");
        send(Message::new().content([1u8; 512]).build(), "out");

        self.send += 2;
    }

    fn handle_message(&mut self, _msg: Message) {
        self.received += 1;
    }

    fn at_sim_end(&mut self) {
        assert_ne!(self.send, self.received)
    }
}

#[test]
#[serial]
fn channel_dropping_message() {
    let mut rt = Sim::new(());
    rt.node("root", DropChanModule::default());

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = Channel::new(
        ObjectPath::new(),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        },
    );
    g_in.connect(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

#[derive(Debug, Default)]
struct BufferChanModule {
    send: usize,
    received: usize,
}

impl Module for BufferChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::new().content([0u8; 512]).build(), "out");
        send(Message::new().content([1u8; 512]).build(), "out");
        send(Message::new().content([1u8; 512]).build(), "out");

        self.send += 3;
    }

    fn handle_message(&mut self, _msg: Message) {
        self.received += 1;
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.send, 3);
        assert_eq!(self.received, 2);
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

    let channel = Channel::new(
        ObjectPath::new(),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::Queue(Some(600)),
        },
    );
    g_in.connect(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

struct SendMessageModule;
impl Module for SendMessageModule {
    fn at_sim_start(&mut self, _stage: usize) {
        schedule_in(Message::new().kind(10).build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, msg: Message) {
        if msg.header().kind == 10 {
            send(Message::new().content("Hello world").build(), "out");
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

    let channel = Channel::new(
        ObjectPath::new(),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        },
    );

    g_in.connect(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

#[derive(Default)]
struct ChannelProbing(Arc<AtomicUsize>);
impl Module for ChannelProbing {
    fn at_sim_start(&mut self, _stage: usize) {
        current()
            .gate("port", 0)
            .unwrap()
            .channel()
            .unwrap()
            .attach_probe(Probe(self.0.clone()));

        send(Message::new().build(), "port")
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.0.load(Ordering::SeqCst), 1);
    }
}

struct Probe(Arc<AtomicUsize>);
impl ChannelProbe for Probe {
    fn on_message_transmit(&mut self, _: &ChannelMetrics, _: &Message) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
#[serial]
fn channel_probes() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = Sim::new(());
    rt.node("alice", ChannelProbing::default());
    rt.node("bob", ChannelProbing::default());

    let alice_port = rt.gate("alice", "port");
    let bob_port = rt.gate("bob", "port");

    let chan = Channel::new(
        ObjectPath::new(),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        },
    );

    alice_port.connect(bob_port, Some(chan));

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}
