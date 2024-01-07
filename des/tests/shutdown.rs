#![cfg(feature = "async")]

use des::{prelude::*, time::sleep};
use serial_test::serial;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[macro_use]
mod common;

struct DropTest {
    heap: Vec<usize>,
    dropper: &'static AtomicUsize,
}

impl DropTest {
    fn new(dropper: &'static AtomicUsize) -> Self {
        Self {
            heap: vec![0],
            dropper,
        }
    }

    fn step(&mut self) -> usize {
        let v = self.heap[self.heap.len() - 1];
        self.heap.push(v + 1);
        v + 1
    }
}

impl Drop for DropTest {
    fn drop(&mut self) {
        println!("DROPPED");
        self.dropper.fetch_add(1, Ordering::SeqCst);
    }
}

struct StatelessModule {}
impl_build_named!(StatelessModule);


impl AsyncModule for StatelessModule {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async {
            let mut drop_test = DropTest::new(&DROPPED_STATELESS_SHUTDOWN);
            loop {
                sleep(Duration::from_secs(1)).await;
                drop_test.step();
            }
        });
    }

    async fn handle_message(&mut self, _msg: Message) {
        shutdown();
    }
}

static DROPPED_STATELESS_SHUTDOWN: AtomicUsize = AtomicUsize::new(0);

#[test]
#[serial]
fn stateless_module_shudown() {
    println!("0");

    DROPPED_STATELESS_SHUTDOWN.store(0, Ordering::SeqCst);
    println!("1");

    let mut rt = NetworkApplication::new(());

    let module = StatelessModule::build_named(ObjectPath::from("RootModule"), &mut rt);
    let gate = module.create_gate("in", GateServiceType::Input);

    rt.register_module(module);
    let mut rt = Builder::seeded(123).build(rt);
    rt.add_message_onto(
        gate,
        Message::new().build(),
        SimTime::from_duration(Duration::from_secs(10)),
    );

    println!("2");
    let _ = rt.run().unwrap();

    println!("3");
    assert_eq!(DROPPED_STATELESS_SHUTDOWN.load(Ordering::SeqCst), 1)
}

struct StatelessModuleRestart {}
impl_build_named!(StatelessModuleRestart);


impl AsyncModule for StatelessModuleRestart {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async {
            let mut drop_test = DropTest::new(&DROPPED_STATLESS_RESTART);
            loop {
                sleep(Duration::from_secs(1)).await;
                drop_test.step();
            }
        });
    }

    async fn handle_message(&mut self, msg: Message) {
        match msg.header().id {
            9 => shutdow_and_restart_at(SimTime::now() + Duration::from_secs(10)),
            10 => shutdown(),
            _ => unreachable!(),
        }
    }
}

static DROPPED_STATLESS_RESTART: AtomicUsize = AtomicUsize::new(0);

#[test]
#[serial]
fn stateless_module_restart() {
    DROPPED_STATLESS_RESTART.store(0, Ordering::SeqCst);

    let mut rt = NetworkApplication::new(());

    let module = StatelessModuleRestart::build_named(ObjectPath::from("RootModule"), &mut rt);
    let gate = module.create_gate("in", GateServiceType::Input);

    rt.register_module(module);
    let mut rt = Builder::seeded(123).build(rt);
    rt.add_message_onto(
        gate.clone(),
        Message::new().id(9).build(),
        SimTime::from_duration(Duration::from_secs(10)),
    );
    rt.add_message_onto(
        gate,
        Message::new().id(10).build(),
        SimTime::from_duration(Duration::from_secs(30)),
    );

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_STATLESS_RESTART.load(Ordering::SeqCst), 2)
}

struct StatefullModule {
    state: usize,
}
impl_build_named!(StatefullModule);


impl AsyncModule for StatefullModule {
    fn new() -> Self {
        Self { state: 0 }
    }

    fn reset(&mut self) {
        assert_eq!(self.state, 10);
        self.state = 5;
    }

    async fn at_sim_start(&mut self, _: usize) {
        self.state = 10;
        tokio::spawn(async {
            let mut drop_test = DropTest::new(&DROPPED_STATFULL_RESTART);
            loop {
                sleep(Duration::from_secs(1)).await;
                drop_test.step();
            }
        });
    }

    async fn handle_message(&mut self, msg: Message) {
        match msg.header().id {
            9 => shutdow_and_restart_at(SimTime::now() + Duration::from_secs(10)),
            10 => shutdown(),
            _ => unreachable!(),
        }
    }

    async fn at_sim_end(&mut self) {
        assert_eq!(self.state, 5)
    }
}

static DROPPED_STATFULL_RESTART: AtomicUsize = AtomicUsize::new(0);

#[test]
#[serial]
fn statefull_module_restart() {
    DROPPED_STATFULL_RESTART.store(0, Ordering::SeqCst);

    let mut rt = NetworkApplication::new(());

    let module = StatefullModule::build_named(ObjectPath::from("RootModule"), &mut rt);
    let gate = module.create_gate("in", GateServiceType::Input);

    rt.register_module(module);
    let mut rt = Builder::seeded(123).build(rt);
    rt.add_message_onto(
        gate.clone(),
        Message::new().id(9).build(),
        SimTime::from_duration(Duration::from_secs(10)),
    );
    rt.add_message_onto(
        gate,
        Message::new().id(10).build(),
        SimTime::from_duration(Duration::from_secs(30)),
    );

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_STATFULL_RESTART.load(Ordering::SeqCst), 2);
}

struct ShutdownViaHandleModule {}
impl_build_named!(ShutdownViaHandleModule);

impl AsyncModule for ShutdownViaHandleModule {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async move {
            let mut drop_test = DropTest::new(&DROPPED_SHUTDOWN_VIA_HANDLE);
            loop {
                sleep(Duration::from_secs(1)).await;
                if drop_test.step() > 10 {
                    shutdown()
                }
            }
        });
    }
}

static DROPPED_SHUTDOWN_VIA_HANDLE: AtomicUsize = AtomicUsize::new(0);

#[test]
#[serial]
fn shutdown_via_async_handle() {
    DROPPED_SHUTDOWN_VIA_HANDLE.store(0, Ordering::SeqCst);

    let mut rt = NetworkApplication::new(());

    let module = ShutdownViaHandleModule::build_named(ObjectPath::from("RootModule"), &mut rt);
    rt.register_module(module);
    let rt = Builder::seeded(123).build(rt);

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_SHUTDOWN_VIA_HANDLE.load(Ordering::SeqCst), 1)
}

struct RestartViaHandleModule {}
impl_build_named!(RestartViaHandleModule);

impl AsyncModule for RestartViaHandleModule {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async move {
            let mut drop_test = DropTest::new(&DROPPED_RESTART_VIA_HANDLE);
            loop {
                sleep(Duration::from_secs(1)).await;
                let v = drop_test.step();

                if v == 10 {
                    if SimTime::now() < SimTime::from_duration(Duration::from_secs(20)) {
                        shutdow_and_restart_at(SimTime::from_duration(Duration::from_secs(30)));
                    } else {
                        shutdown();
                    }
                }
            }
        });
    }
}

static DROPPED_RESTART_VIA_HANDLE: AtomicUsize = AtomicUsize::new(0);

#[test]
#[serial]
fn restart_via_async_handle() {
    DROPPED_RESTART_VIA_HANDLE.store(0, Ordering::SeqCst);

    let mut rt = NetworkApplication::new(());

    let module = RestartViaHandleModule::build_named(ObjectPath::from("RootModule"), &mut rt);
    rt.register_module(module);
    let rt = Builder::seeded(123).build(rt);

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_RESTART_VIA_HANDLE.load(Ordering::SeqCst), 2)
}

struct CountDropsMessage {
    counter: Arc<AtomicUsize>,
}
impl MessageBody for CountDropsMessage {
    fn byte_len(&self) -> usize {
        1
    }
}
impl Drop for CountDropsMessage {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
    }
}

struct WillIgnoreInncomingInDowntime {
    received: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}
impl_build_named!(WillIgnoreInncomingInDowntime);

impl Module for WillIgnoreInncomingInDowntime {
    fn new() -> Self {
        Self {
            received: Arc::new(AtomicUsize::new(0)),
            drops: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        if SimTime::now().as_secs() == 0 {
            // schedule events for seconds 1..=10
            for i in 1..=10 {
                schedule_in(
                    Message::new()
                        .content(CountDropsMessage {
                            counter: self.drops.clone(),
                        })
                        .build(),
                    Duration::from_secs(i),
                );
            }
        }
    }

    fn handle_message(&mut self, mut msg: Message) {
        self.received.fetch_add(1, Ordering::SeqCst);
        if SimTime::now().as_secs() == 6 {
            shutdow_and_restart_in(Duration::from_secs_f64(2.5));
            // will miss incoming messages '7 and '8
        }

        // Forget the message, aka assign an temp counter
        msg.content_mut::<CountDropsMessage>().counter = Arc::new(AtomicUsize::new(0));
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.received.load(Ordering::SeqCst), 8);
        assert_eq!(self.drops.load(Ordering::SeqCst), 2);
    }
}

#[test]
#[serial]
fn shutdown_will_ignore_incoming() {
    let mut rt = NetworkApplication::new(());

    let module =
        WillIgnoreInncomingInDowntime::build_named(ObjectPath::from("RootModule"), &mut rt);
    rt.register_module(module);
    let rt = Builder::seeded(123).build(rt);

    let _ = rt.run().unwrap();
}

struct EndNode {
    sent: usize,
    recv: usize,
    drops: Arc<AtomicUsize>,
}
impl_build_named!(EndNode);

impl Module for EndNode {
    fn new() -> Self {
        Self {
            sent: 0,
            recv: 0,
            drops: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn at_sim_start(&mut self, _: usize) {
        schedule_in(Message::new().kind(1).build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, mut msg: Message) {
        match msg.header().kind {
            1 => {
                if SimTime::now().as_secs() > 10 {
                    return;
                }

                self.sent += 1;
                send(
                    Message::new()
                        .kind(2)
                        .content(CountDropsMessage {
                            counter: self.drops.clone(),
                        })
                        .build(),
                    "out",
                );
                schedule_in(Message::new().kind(1).build(), Duration::from_secs(1));
            }
            2 => {
                self.recv += 1;

                // forget the message drop counter;
                msg.content_mut::<CountDropsMessage>().counter = Arc::new(AtomicUsize::new(0));
            }
            _ => unreachable!(),
        }
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.sent, 10);
        assert_eq!(self.recv, 7);

        assert_eq!(self.drops.load(Ordering::SeqCst), 3);
    }
}

struct Transit;
impl_build_named!(Transit);

impl Module for Transit {
    fn new() -> Self {
        Self
    }

    fn at_sim_start(&mut self, _stage: usize) {
        if SimTime::now() == SimTime::ZERO {
            schedule_in(Message::new().build(), Duration::from_secs_f64(5.5));
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        // happens at 5.5 so '6 '7 '8 will be lost
        shutdow_and_restart_in(Duration::from_secs(3));
    }
}

#[test]
#[serial]
fn shutdown_will_drop_transiting() {
    // Logger::new().set_logger();
    let mut app = NetworkApplication::new(());

    let ping = EndNode::build_named(ObjectPath::from("ping"), &mut app);
    let pong = EndNode::build_named(ObjectPath::from("pong"), &mut app);
    let transit = Transit::build_named(ObjectPath::from("transit"), &mut app);

    let ping_in = ping.create_gate("in", GateServiceType::Input);
    let ping_out = ping.create_gate("out", GateServiceType::Output);
    let pong_in = pong.create_gate("in", GateServiceType::Input);
    let pong_out = pong.create_gate("out", GateServiceType::Output);

    let i_to_o = transit.create_gate("i_to_o", GateServiceType::Undefined);
    let o_to_i = transit.create_gate("i_to_o", GateServiceType::Undefined);

    ping_out.set_next_gate(i_to_o.clone());
    i_to_o.set_next_gate(pong_in);

    pong_out.set_next_gate(o_to_i.clone());
    o_to_i.set_next_gate(ping_in);

    app.register_module(ping);
    app.register_module(pong);
    app.register_module(transit);

    let rt = Builder::seeded(123).max_itr(500).build(app);
    let _ = rt.run().unwrap();
}

#[test]
#[serial]
fn shutdown_will_drop_transiting_delayed_channels() {
    // Logger::new().set_logger();
    let mut app = NetworkApplication::new(());

    let ping = EndNode::build_named(ObjectPath::from("ping"), &mut app);
    let pong = EndNode::build_named(ObjectPath::from("pong"), &mut app);
    let transit = Transit::build_named(ObjectPath::from("transit"), &mut app);

    let ping_in = ping.create_gate("in", GateServiceType::Input);
    let ping_out = ping.create_gate("out", GateServiceType::Output);
    let pong_in = pong.create_gate("in", GateServiceType::Input);
    let pong_out = pong.create_gate("out", GateServiceType::Output);

    let i_to_o = transit.create_gate("i_to_o", GateServiceType::Undefined);
    let o_to_i = transit.create_gate("i_to_o", GateServiceType::Undefined);

    let ch_to_o = Channel::new(
        ping.path().appended_channel("to_o"),
        ChannelMetrics {
            bitrate: 100_000,
            latency: Duration::from_secs_f64(0.004),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        },
    );
    let ch_to_i = Channel::new(
        ping.path().appended_channel("to_o"),
        ChannelMetrics {
            bitrate: 100_000,
            latency: Duration::from_secs_f64(0.004),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        },
    );

    ping_out.set_next_gate(i_to_o.clone());
    ping_out.set_channel(ch_to_o);
    i_to_o.set_next_gate(pong_in);

    pong_out.set_next_gate(o_to_i.clone());
    pong_out.set_channel(ch_to_i);
    o_to_i.set_next_gate(ping_in);

    app.register_module(ping);
    app.register_module(pong);
    app.register_module(transit);

    let rt = Builder::seeded(123).max_itr(500).build(app);
    let _ = rt.run().unwrap();
}
