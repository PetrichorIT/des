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

#[macro_use]
mod common;

struct DropChanModule {
    send: usize,
    received: usize,
}
impl_build_named!(DropChanModule);

impl Module for DropChanModule {
    fn new() -> Self {
        Self {
            send: 0,
            received: 0,
        }
    }

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
    let mut rt = NetworkApplication::new(());

    let module = DropChanModule::build_named(ObjectPath::from("root".to_string()), &mut rt);

    let g_in = module.create_gate("in");
    let g_out = module.create_gate("out");

    let channel = Channel::new(
        ObjectPath::appended_channel(&module.path(), "chan"),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        },
    );
    g_in.connect(g_out, Some(channel));

    rt.register_module(module);

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

#[derive(Debug)]
struct BufferChanModule {
    send: usize,
    received: usize,
}
impl_build_named!(BufferChanModule);

impl Module for BufferChanModule {
    fn new() -> Self {
        Self {
            send: 0,
            received: 0,
        }
    }

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

    let mut rt = NetworkApplication::new(());

    let module = BufferChanModule::build_named(ObjectPath::from("root".to_string()), &mut rt);

    let g_in = module.create_gate("in");
    let g_out = module.create_gate("out");

    let channel = Channel::new(
        ObjectPath::appended_channel(&module.path(), "chan"),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::Queue(Some(600)),
        },
    );
    g_in.connect(g_out, Some(channel));

    rt.register_module(module);

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

struct SendMessageModule;
impl_build_named!(SendMessageModule);
impl Module for SendMessageModule {
    fn new() -> Self {
        Self
    }

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

    let mut rt = NetworkApplication::new(());

    let module = SendMessageModule::build_named(ObjectPath::from("root".to_string()), &mut rt);

    let g_in = module.create_gate("in");
    let g_out = module.create_gate("out");

    let channel = Channel::new(
        ObjectPath::appended_channel(&module.path(), "chan"),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        },
    );

    g_in.connect(g_out, Some(channel));
    rt.register_module(module);

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

struct ChannelProbing(Arc<AtomicUsize>);
impl_build_named!(ChannelProbing);
impl Module for ChannelProbing {
    fn new() -> Self {
        Self(Arc::new(AtomicUsize::new(0)))
    }
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

    let mut rt = NetworkApplication::new(());

    let alice = ChannelProbing::build_named(ObjectPath::from("alice".to_string()), &mut rt);
    let bob = ChannelProbing::build_named(ObjectPath::from("bob".to_string()), &mut rt);

    let alice_port = alice.create_gate("port");
    let bob_port = bob.create_gate("port");

    let chan = Channel::new(
        ObjectPath::appended_channel(&alice.path(), "chan"),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        },
    );

    alice_port.connect(bob_port, Some(chan));

    rt.register_module(alice);
    rt.register_module(bob);

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}
