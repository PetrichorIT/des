#![cfg(feature = "async")]

use des::net::{BuildContext, __Buildable0};
use des::prelude::*;
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};

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

#[NdlModule]
struct StatelessModule {}

#[async_trait::async_trait]
impl AsyncModule for StatelessModule {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async {
            let mut drop_test = DropTest::new(&DROPPED_STATELESS_SHUTDOWN);
            loop {
                tokio::sim::time::sleep(Duration::from_secs(1)).await;
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
    DROPPED_STATELESS_SHUTDOWN.store(0, Ordering::SeqCst);

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module =
        StatelessModule::build_named(ObjectPath::root_module("RootModule".to_string()), &mut cx);
    let gate = module.create_gate("in", GateServiceType::Input);

    rt.create_module(module);
    let mut rt = Runtime::new(rt);
    rt.add_message_onto(
        gate,
        Message::new().build(),
        SimTime::from_duration(Duration::from_secs(10)),
    );

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_STATELESS_SHUTDOWN.load(Ordering::SeqCst), 1)
}

#[NdlModule]
struct StatelessModuleRestart {}

#[async_trait::async_trait]
impl AsyncModule for StatelessModuleRestart {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async {
            let mut drop_test = DropTest::new(&DROPPED_STATLESS_RESTART);
            loop {
                tokio::sim::time::sleep(Duration::from_secs(1)).await;
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

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module = StatelessModuleRestart::build_named(
        ObjectPath::root_module("RootModule".to_string()),
        &mut cx,
    );
    let gate = module.create_gate("in", GateServiceType::Input);

    rt.create_module(module);
    let mut rt = Runtime::new(rt);
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

#[NdlModule]
struct StatefullModule {
    state: usize,
}

#[async_trait::async_trait]
impl AsyncModule for StatefullModule {
    fn new() -> Self {
        Self { state: 0 }
    }
    async fn at_sim_start(&mut self, _: usize) {
        self.state = 10;
        tokio::spawn(async {
            let mut drop_test = DropTest::new(&DROPPED_STATFULL_RESTART);
            loop {
                tokio::sim::time::sleep(Duration::from_secs(1)).await;
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

    fn at_restart(&mut self) {
        assert_eq!(self.state, 10);
        self.state = 5;
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

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module =
        StatefullModule::build_named(ObjectPath::root_module("RootModule".to_string()), &mut cx);
    let gate = module.create_gate("in", GateServiceType::Input);

    rt.create_module(module);
    let mut rt = Runtime::new(rt);
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

#[NdlModule]
struct ShutdownViaHandleModule {}

#[async_trait::async_trait]
impl AsyncModule for ShutdownViaHandleModule {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async move {
            let mut drop_test = DropTest::new(&DROPPED_SHUTDOWN_VIA_HANDLE);
            loop {
                tokio::sim::time::sleep(Duration::from_secs(1)).await;
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

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module = ShutdownViaHandleModule::build_named(
        ObjectPath::root_module("RootModule".to_string()),
        &mut cx,
    );
    rt.create_module(module);
    let rt = Runtime::new(rt);

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_SHUTDOWN_VIA_HANDLE.load(Ordering::SeqCst), 1)
}

#[NdlModule]
struct RestartViaHandleModule {}

#[async_trait::async_trait]
impl AsyncModule for RestartViaHandleModule {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async move {
            let mut drop_test = DropTest::new(&DROPPED_RESTART_VIA_HANDLE);
            loop {
                tokio::sim::time::sleep(Duration::from_secs(1)).await;
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

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module = RestartViaHandleModule::build_named(
        ObjectPath::root_module("RootModule".to_string()),
        &mut cx,
    );
    rt.create_module(module);
    let rt = Runtime::new(rt);

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_RESTART_VIA_HANDLE.load(Ordering::SeqCst), 2)
}
