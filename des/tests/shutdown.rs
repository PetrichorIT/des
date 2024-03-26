#![cfg(feature = "async")]

use des::{net::ModuleFn, prelude::*, time::sleep};
use serial_test::serial;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

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

struct StatelessModule;

impl AsyncModule for StatelessModule {
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

    let mut rt = Sim::new(());
    rt.node("root", StatelessModule);
    let gate = rt.gate("root", "in");

    let mut rt = Builder::seeded(123).build(rt);
    rt.add_message_onto(
        gate,
        Message::new().build(),
        SimTime::from_duration(Duration::from_secs(10)),
    );

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_STATELESS_SHUTDOWN.load(Ordering::SeqCst), 1)
}

struct StatelessModuleRestart;

impl AsyncModule for StatelessModuleRestart {
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

    let mut rt = Sim::new(());
    rt.node("root", StatelessModuleRestart);
    let gate = rt.gate("root", "in");

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

#[derive(Default)]
struct StatefullModule {
    state: usize,
}

impl AsyncModule for StatefullModule {
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

    let mut rt = Sim::new(());
    rt.node("root", StatefullModule::default());
    let gate = rt.gate("root", "in");

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

struct ShutdownViaHandleModule;

impl AsyncModule for ShutdownViaHandleModule {
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

    let mut rt = Sim::new(());
    rt.node("root", ShutdownViaHandleModule);

    let rt = Builder::seeded(123).build(rt);

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_SHUTDOWN_VIA_HANDLE.load(Ordering::SeqCst), 1)
}

struct RestartViaHandleModule;

impl AsyncModule for RestartViaHandleModule {
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

    let mut rt = Sim::new(());
    rt.node("root", RestartViaHandleModule);

    let rt = Builder::seeded(123).build(rt);

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_RESTART_VIA_HANDLE.load(Ordering::SeqCst), 2)
}

#[derive(Clone)]
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

#[derive(Default)]
struct WillIgnoreInncomingInDowntime {
    received: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

impl Module for WillIgnoreInncomingInDowntime {
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
    let mut rt = Sim::new(());
    rt.node("root", WillIgnoreInncomingInDowntime::default());

    let rt = Builder::seeded(123).build(rt);

    let _ = rt.run().unwrap();
}

#[derive(Default)]
struct EndNode {
    sent: usize,
    recv: usize,
    drops: Arc<AtomicUsize>,
}

impl Module for EndNode {
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
                    "port",
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

impl Module for Transit {
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
    let mut app = Sim::new(());
    app.node("ping", EndNode::default());
    app.node("pong", EndNode::default());
    app.node("transit", Transit);

    let ping = app.gate("ping", "port");
    let pong = app.gate("pong", "port");
    let con = app.gate("transit", "connector");

    ping.connect(con.clone(), None);
    con.connect(pong, None);

    let rt = Builder::seeded(123).max_itr(500).build(app);
    let _ = rt.run().unwrap();
}

#[test]
#[serial]
fn shutdown_will_drop_transiting_delayed_channels() {
    // Logger::new().set_logger();
    let mut app = Sim::new(());

    app.node("ping", EndNode::default());
    app.node("pong", EndNode::default());
    app.node("transit", Transit);

    let ping = app.gate("ping", "port");
    let pong = app.gate("pong", "port");
    let con = app.gate("transit", "connector");

    ping.connect(
        con.clone(),
        Some(Channel::new(ChannelMetrics {
            bitrate: 100_000,
            latency: Duration::from_secs_f64(0.004),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        })),
    );
    con.connect(
        pong,
        Some(Channel::new(ChannelMetrics {
            bitrate: 100_000,
            latency: Duration::from_secs_f64(0.004),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        })),
    );

    let rt = Builder::seeded(123).max_itr(500).build(app);
    let _ = rt.run().unwrap();
}

#[test]
#[serial]
fn shutdown_prevents_accessing_parents() {
    let mut sim = Sim::new(());
    sim.node("a", ModuleFn::new(
        || schedule_in(Message::new().build(), Duration::from_secs(10)),
        |_, _| {
            let err = current().child("b").unwrap_err();
            assert_eq!(err, ModuleReferencingError::CurrentlyInactive("The child module 'b' of 'a' is currently shut down, thus cannot be accessed".to_string()));
        }
    ));
    sim.node(
        "a.b",
        ModuleFn::new(
            || schedule_in(Message::new().build(), Duration::from_secs(5)),
            |_, _| {
                shutdown();
            },
        ),
    );
    sim.node(
        "a.b.c",
        ModuleFn::new(
            || schedule_in(Message::new().build(), Duration::from_secs(10)),
            |_, _| {
                let err = current().parent().unwrap_err();
                assert_eq!(err, ModuleReferencingError::CurrentlyInactive("The parent module of 'a.b.c' is currently shut down, thus cannot be accessed".to_string()));
            },
        ),
    );

    let _ = Builder::seeded(123).build(sim).run();
}
