#![cfg(feature = "net")]
use des::prelude::*;
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
    let mut rt = NetworkRuntime::new(());

    let module = DropChanModule::build_named(ObjectPath::from("root".to_string()), &mut rt);

    let g_in = module.create_gate("in", GateServiceType::Input);
    let g_out = module.create_gate("out", GateServiceType::Output);

    let channel = Channel::new(
        ObjectPath::appended_channel(&module.path(), "chan"),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            queuesize: 0,
            cost: 1.0,
        },
    );

    g_out.set_channel(channel);
    g_out.set_next_gate(g_in);

    rt.create_module(module);

    let rt = Runtime::new(rt);
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

    let mut rt = NetworkRuntime::new(());

    let module = BufferChanModule::build_named(ObjectPath::from("root".to_string()), &mut rt);

    let g_in = module.create_gate("in", GateServiceType::Input);
    let g_out = module.create_gate("out", GateServiceType::Output);

    let channel = Channel::new(
        ObjectPath::appended_channel(&module.path(), "chan"),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            queuesize: 600,
            cost: 1.0,
        },
    );

    g_out.set_channel(channel);
    g_out.set_next_gate(g_in);

    rt.create_module(module);

    let rt = Runtime::new(rt);
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
            let gate = gate("out", 0).unwrap();
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

    let mut rt = NetworkRuntime::new(());

    let module = SendMessageModule::build_named(ObjectPath::from("root".to_string()), &mut rt);

    let g_in = module.create_gate("in", GateServiceType::Input);
    let g_out = module.create_gate("out", GateServiceType::Output);

    let channel = Channel::new(
        ObjectPath::appended_channel(&module.path(), "chan"),
        ChannelMetrics {
            bitrate: 1000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            queuesize: 0,
            cost: 1.0,
        },
    );

    g_out.set_channel(channel);
    g_out.set_next_gate(g_in);

    rt.create_module(module);

    let rt = Runtime::new(rt);
    let _ = rt.run();
}
