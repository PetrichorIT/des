#![cfg(feature = "net")]
use des::net::__Buildable0;
use des::{net::BuildContext, prelude::*};
use serial_test::serial;

#[NdlModule]
struct DropChanModule {
    send: usize,
    received: usize,
}

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
    let mut cx = BuildContext::new(&mut rt);

    let module = DropChanModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);

    let g_in = module.create_gate("in", GateServiceType::Input);
    let g_out = module.create_gate("out", GateServiceType::Output);

    let channel = Channel::new(
        ObjectPath::channel_with("chan", &module.path()),
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

#[NdlModule]
#[derive(Debug)]
struct BufferChanModule {
    send: usize,
    received: usize,
}

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
    let mut cx = BuildContext::new(&mut rt);

    let module =
        BufferChanModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);

    let g_in = module.create_gate("in", GateServiceType::Input);
    let g_out = module.create_gate("out", GateServiceType::Output);

    let channel = Channel::new(
        ObjectPath::channel_with("chan", &module.path()),
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
