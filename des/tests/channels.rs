#![cfg(feature = "net")]
use des::prelude::*;
use serial_test::serial;

#[NdlModule]
struct DropChanModule {
    send: usize,
    received: usize,
}

impl NameableModule for DropChanModule {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            send: 0,
            received: 0,
        }
    }
}

impl Module for DropChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        self.send(Message::new().content([0u8; 512]).build(), "out");
        self.send(Message::new().content([1u8; 512]).build(), "out");

        self.send += 2;
    }

    fn handle_message(&mut self, _msg: Message) {
        self.received += 1;
    }

    fn at_sim_end(&mut self) {
        assert_ne!(self.send, self.received)
    }
}

#[serial]
#[test]
fn channel_dropping_message() {
    let mut rt = NetworkRuntime::new(());
    let mut module = DropChanModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("root".to_string()),
        PtrWeak::from_strong(&rt.globals()),
    ));

    let g_in = module.create_gate("in", GateServiceType::Input, &mut rt);
    let mut g_out = module.create_gate("out", GateServiceType::Output, &mut rt);

    let channel = Channel::new(
        ObjectPath::channel_with("chan", module.path()),
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
struct BufferChanModule {
    send: usize,
    received: usize,
}

impl NameableModule for BufferChanModule {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            send: 0,
            received: 0,
        }
    }
}

impl Module for BufferChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        self.send(Message::new().content([0u8; 512]).build(), "out");
        self.send(Message::new().content([1u8; 512]).build(), "out");
        self.send(Message::new().content([1u8; 512]).build(), "out");

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

#[serial]
#[test]
fn channel_buffering_message() {
    let mut rt = NetworkRuntime::new(());
    let mut module = BufferChanModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("root".to_string()),
        PtrWeak::from_strong(&rt.globals()),
    ));

    let g_in = module.create_gate("in", GateServiceType::Input, &mut rt);
    let mut g_out = module.create_gate("out", GateServiceType::Output, &mut rt);

    let channel = Channel::new(
        ObjectPath::channel_with("chan", module.path()),
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
