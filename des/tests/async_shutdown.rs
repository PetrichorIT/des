#![cfg(feature = "async")]
#![cfg(not(feature = "async-sharedrt"))]

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
        self.shutdown(None);
    }
}

static DROPPED_STATELESS_SHUTDOWN: AtomicUsize = AtomicUsize::new(0);

#[serial]
#[test]
fn stateless_module_shudown() {
    DROPPED_STATELESS_SHUTDOWN.store(0, Ordering::SeqCst);

    let mut rt = NetworkRuntime::new(());
    let mut module = StatelessModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let gate = module.create_gate("in", GateServiceType::Input, &mut rt);

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
        match msg.meta().id {
            9 => self.shutdown(Some(SimTime::now() + Duration::from_secs(10))),
            10 => self.shutdown(None),
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
    let mut module = StatelessModuleRestart::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let gate = module.create_gate("in", GateServiceType::Input, &mut rt);

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

impl NameableModule for StatefullModule {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            state: 0,
        }
    }
}

#[async_trait::async_trait]
impl AsyncModule for StatefullModule {
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
        match msg.meta().id {
            9 => self.shutdown(Some(SimTime::now() + Duration::from_secs(10))),
            10 => self.shutdown(None),
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
    let mut module = StatefullModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let gate = module.create_gate("in", GateServiceType::Input, &mut rt);

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
    async fn at_sim_start(&mut self, _: usize) {
        let handle = self.async_handle();
        tokio::spawn(async move {
            let mut drop_test = DropTest::new(&DROPPED_SHUTDOWN_VIA_HANDLE);
            loop {
                tokio::sim::time::sleep(Duration::from_secs(1)).await;
                if drop_test.step() > 10 {
                    handle.shutdown(None);
                }
            }
        });
    }
}

static DROPPED_SHUTDOWN_VIA_HANDLE: AtomicUsize = AtomicUsize::new(0);

#[serial]
#[test]
fn shutdown_via_async_handle() {
    DROPPED_SHUTDOWN_VIA_HANDLE.store(0, Ordering::SeqCst);

    let mut rt = NetworkRuntime::new(());
    let module = ShutdownViaHandleModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    rt.create_module(module);
    let rt = Runtime::new(rt);

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_SHUTDOWN_VIA_HANDLE.load(Ordering::SeqCst), 1)
}

#[NdlModule]
struct RestartViaHandleModule {}

#[async_trait::async_trait]
impl AsyncModule for RestartViaHandleModule {
    async fn at_sim_start(&mut self, _: usize) {
        let handle = self.async_handle();
        tokio::spawn(async move {
            let mut drop_test = DropTest::new(&DROPPED_RESTART_VIA_HANDLE);
            loop {
                tokio::sim::time::sleep(Duration::from_secs(1)).await;
                let v = drop_test.step();

                if v == 10 {
                    if SimTime::now() < SimTime::from_duration(Duration::from_secs(20)) {
                        handle.shutdown(Some(SimTime::from_duration(Duration::from_secs(30))));
                    } else {
                        handle.shutdown(None);
                    }
                }
            }
        });
    }
}

static DROPPED_RESTART_VIA_HANDLE: AtomicUsize = AtomicUsize::new(0);

#[serial]
#[test]
fn restart_via_async_handle() {
    DROPPED_RESTART_VIA_HANDLE.store(0, Ordering::SeqCst);

    let mut rt = NetworkRuntime::new(());
    let module = RestartViaHandleModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    rt.create_module(module);
    let rt = Runtime::new(rt);

    let _ = rt.run().unwrap();
    assert_eq!(DROPPED_RESTART_VIA_HANDLE.load(Ordering::SeqCst), 2)
}
