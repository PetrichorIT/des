#![cfg(feature = "async")]
use std::sync::{
    atomic::{AtomicBool, AtomicUsize},
    Arc,
};

use async_trait::async_trait;
use des::prelude::*;
use tokio::{
    sync::{
        mpsc::{channel, Sender},
        Semaphore,
    },
    task::JoinHandle,
};

use serial_test::serial;

// # Test case
// The module behaves like a sync module, not creating any more
// futures than the async call itself.

#[NdlModule]
struct QuasaiSyncModule {
    counter: usize,
}

impl NameableModule for QuasaiSyncModule {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            counter: 0,
        }
    }
}

#[async_trait]
impl AsyncModule for QuasaiSyncModule {
    async fn handle_message(&mut self, msg: Message) {
        println!("[{}] Received msg: {}", self.name(), msg.meta().id);
        self.counter += msg.meta().id as usize;
    }
}

#[test]
#[serial]
fn quasai_sync_non_blocking() {
    let mut rt = NetworkRuntime::new(());
    let mut module = QuasaiSyncModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    let gate_a = module.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module);

    let mut module_b = QuasaiSyncModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("OtherRootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    let gate_b = module_b.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module_b);

    let mut rt = Runtime::new(rt);

    rt.add_message_onto(gate_a.clone(), Message::new().id(1).build(), SimTime::ZERO);
    rt.add_message_onto(gate_a, Message::new().id(2).build(), SimTime::ZERO);

    rt.add_message_onto(gate_b.clone(), Message::new().id(1).build(), SimTime::ZERO);
    rt.add_message_onto(gate_b.clone(), Message::new().id(2).build(), SimTime::ZERO);
    rt.add_message_onto(gate_b, Message::new().id(3).build(), SimTime::ZERO);

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            event_count,
        } => {
            assert_eq!(time, SimTime::ZERO);
            assert_eq!(event_count, 11);

            let m1 = app
                .module(|m| dbg!(m.module_core().name()) == "RootModule")
                .unwrap()
                .self_as::<QuasaiSyncModule>()
                .unwrap();

            assert_eq!(m1.counter, 1 + 2);

            let m2 = app
                .module(|m| m.module_core().name() == "OtherRootModule")
                .unwrap()
                .self_as::<QuasaiSyncModule>()
                .unwrap();

            assert_eq!(m2.counter, 1 + 2 + 3)
        }
        _ => assert!(false, "Expected runtime to finish"),
    }
}

// # Test case
// A module has 3 permantent tasks that each forward
// the message, the final one incrementing a module bound
// tracker
// The tasks shutdown with a shutdown message

#[NdlModule]
struct MutipleTasksModule {
    handles: Vec<JoinHandle<()>>,
    sender: Option<Sender<Message>>,
    result: Arc<AtomicUsize>,
}

impl NameableModule for MutipleTasksModule {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            handles: Vec::new(),
            sender: None,
            result: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl AsyncModule for MutipleTasksModule {
    async fn at_sim_start(&mut self, _: usize) {
        let (txa, mut rxa) = channel::<Message>(8);
        let (txb, mut rxb) = channel(8);
        let (txc, mut rxc) = channel(8);

        let result = self.result.clone();

        let ta = tokio::spawn(async move {
            while let Some(v) = rxa.recv().await {
                let k = v.meta().kind;
                txb.send(v).await.unwrap();

                if k == 42 {
                    rxa.close();
                    txb.closed().await;
                }
            }
        });

        let tb = tokio::spawn(async move {
            while let Some(v) = rxb.recv().await {
                let k = v.meta().kind;
                txc.send(v).await.unwrap();

                if k == 42 {
                    rxb.close();
                    txc.closed().await;
                }
            }
        });

        let tc = tokio::spawn(async move {
            while let Some(v) = rxc.recv().await {
                let k = v.meta().kind;
                result.fetch_add(v.meta().id as usize, std::sync::atomic::Ordering::SeqCst);

                if k == 42 {
                    rxc.close();
                }
            }
        });

        self.sender = Some(txa);
        self.handles.push(ta);
        self.handles.push(tb);
        self.handles.push(tc);
    }

    async fn at_sim_end(&mut self) {
        self.sender
            .take()
            .unwrap()
            .send(Message::new().kind(42).build())
            .await
            .unwrap();

        for join in self.handles.drain(..) {
            join.await.unwrap()
        }

        self.result
            .fetch_add(100, std::sync::atomic::Ordering::SeqCst);
    }

    async fn handle_message(&mut self, msg: Message) {
        self.sender.as_ref().unwrap().send(msg).await.unwrap()
    }
}

#[test]
#[serial]
fn mutiple_active_tasks() {
    let mut rt = NetworkRuntime::new(());
    let mut module_a = MutipleTasksModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    let gate_a = module_a.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module_a);

    let mut rt = Runtime::new(rt);

    rt.add_message_onto(gate_a.clone(), Message::new().id(1).build(), SimTime::ZERO);
    rt.add_message_onto(gate_a, Message::new().id(2).build(), SimTime::ZERO);

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            event_count,
        } => {
            assert_eq!(time, SimTime::ZERO);

            // SimStart + 2 * (Gate + HandleMessage)
            assert_eq!(event_count, 5);

            let m1 = app
                .module(|m| m.module_core().name() == "RootModule")
                .unwrap()
                .self_as::<MutipleTasksModule>()
                .unwrap();

            assert_eq!(m1.result.load(std::sync::atomic::Ordering::SeqCst), 100 + 3);
        }
        _ => assert!(false, "Expected runtime to finish"),
    }
}

// # Test case
// A module sleeps upon receiving a message,
// This sleeps do NOT interfere with recv()

#[NdlModule]
struct TimeSleepModule {
    counter: usize,
}

impl NameableModule for TimeSleepModule {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            counter: 0,
        }
    }
}

#[async_trait]
impl AsyncModule for TimeSleepModule {
    async fn handle_message(&mut self, msg: Message) {
        let wait_time = msg.meta().kind as u64;
        println!("[{}] Waiting for timer", SimTime::now());
        tokio::time::sleep(Duration::from_secs(wait_time)).await;
        println!(
            "[{}] Done waiting for id: {}",
            SimTime::now(),
            msg.meta().id
        );
        self.counter += msg.meta().id as usize
    }
}

#[test]
#[serial]
fn one_module_timers() {
    let mut rt = NetworkRuntime::new(());
    let mut module_a = TimeSleepModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    let gate_a = module_a.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module_a);

    let mut rt = Runtime::new(rt);

    rt.add_message_onto(
        gate_a.clone(),
        Message::new().id(1).kind(1).build(),
        SimTime::ZERO,
    );
    rt.add_message_onto(
        gate_a,
        Message::new().id(2).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(2, 0)),
    );

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            event_count,
        } => {
            assert_eq!(time, 4.0);

            assert_eq!(event_count, 7);

            let m1 = app
                .module(|m| m.module_core().name() == "RootModule")
                .unwrap()
                .self_as::<TimeSleepModule>()
                .unwrap();

            assert_eq!(m1.counter, 3);
        }
        _ => assert!(false, "Expected runtime to finish"),
    }
}

// # Test case
// The module sleeps on message receival
// The sleeps should delay the next recv.

#[test]
#[serial]
fn one_module_delayed_recv() {
    let mut rt = NetworkRuntime::new(());
    let mut module_a = TimeSleepModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    let gate_a = module_a.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module_a);

    let mut rt = Runtime::new(rt);

    rt.add_message_onto(
        gate_a.clone(),
        Message::new().id(1).kind(2).build(),
        SimTime::ZERO,
    );
    rt.add_message_onto(
        gate_a,
        Message::new().id(2).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(2, 0)),
    );

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            event_count,
        } => {
            assert_eq!(time, 4.0);

            assert_eq!(event_count, 7);

            let m1 = app
                .module(|m| m.module_core().name() == "RootModule")
                .unwrap()
                .self_as::<TimeSleepModule>()
                .unwrap();

            assert_eq!(m1.counter, 3);
        }
        _ => assert!(false, "Expected runtime to finish"),
    }
}

// # Test case
// Mutiple Modules delay themself with sleeps

#[test]
#[serial]
fn mutiple_module_delayed_recv() {
    let mut rt = NetworkRuntime::new(());

    let mut module_a = TimeSleepModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let gate_a = module_a.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module_a);

    let mut module_b = TimeSleepModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("OtherRootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let gate_b = module_b.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module_b);

    let mut rt = Runtime::new(rt);

    rt.add_message_onto(
        gate_a.clone(),
        Message::new().id(1).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_a,
        Message::new().id(2).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(2, 0)),
    );

    rt.add_message_onto(
        gate_b.clone(),
        Message::new().id(10).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_b,
        Message::new().id(20).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(2, 0)),
    );

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            event_count,
        } => {
            assert_eq!(time, 4.0);

            assert_eq!(event_count, 17);

            let m1 = app
                .module(|m| m.module_core().name() == "RootModule")
                .unwrap()
                .self_as::<TimeSleepModule>()
                .unwrap();

            assert_eq!(m1.counter, 3);

            let m2 = app
                .module(|m| m.module_core().name() == "OtherRootModule")
                .unwrap()
                .self_as::<TimeSleepModule>()
                .unwrap();

            assert_eq!(m2.counter, 30);
        }
        _ => assert!(false, "Expected runtime to finish"),
    }
}

#[NdlModule]
struct SemaphoreModule {
    semaphore: Arc<Semaphore>,
    handle: Option<JoinHandle<()>>,
    result: Arc<AtomicBool>,
}

impl NameableModule for SemaphoreModule {
    fn named(core: ModuleCore) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(0)),
            handle: None,
            result: Arc::new(AtomicBool::new(false)),
            __core: core,
        }
    }
}

#[async_trait]
impl AsyncModule for SemaphoreModule {
    async fn at_sim_start(&mut self, _: usize) {
        let sem = self.semaphore.clone();
        let res = self.result.clone();
        self.handle = Some(tokio::spawn(async move {
            let premit = sem.acquire_many(5).await.unwrap();
            println!("[{}] Aquired semaphore", SimTime::now());
            res.fetch_or(true, std::sync::atomic::Ordering::SeqCst);
            drop(premit)
        }));
    }

    async fn handle_message(&mut self, msg: Message) {
        self.semaphore.add_permits(msg.meta().kind as usize);
    }
}

#[test]
#[serial]
fn semaphore_in_waiting_task() {
    let mut rt = NetworkRuntime::new(());

    let mut module_a = SemaphoreModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let gate_a = module_a.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module_a);

    let mut module_b = SemaphoreModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("OtherRootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let gate_b = module_b.create_gate("in", GateServiceType::Input, &mut rt);
    rt.create_module(module_b);

    let mut rt = Runtime::new(rt);

    rt.add_message_onto(
        gate_a.clone(),
        Message::new().id(1).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_a,
        Message::new().id(2).kind(3).build(),
        SimTime::duration_since_zero(Duration::new(2, 0)),
    );

    rt.add_message_onto(
        gate_b.clone(),
        Message::new().id(10).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_b.clone(),
        Message::new().id(20).kind(2).build(),
        SimTime::duration_since_zero(Duration::new(2, 0)),
    );
    rt.add_message_onto(
        gate_b,
        Message::new().id(20).kind(1).build(),
        SimTime::duration_since_zero(Duration::new(3, 0)),
    );

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            event_count,
        } => {
            assert_eq!(time, 3.0);

            assert_eq!(event_count, 11);

            let m1 = app
                .module(|m| m.module_core().name() == "RootModule")
                .unwrap()
                .self_as::<SemaphoreModule>()
                .unwrap();

            assert!(m1.result.load(std::sync::atomic::Ordering::SeqCst));

            let m2 = app
                .module(|m| m.module_core().name() == "OtherRootModule")
                .unwrap()
                .self_as::<SemaphoreModule>()
                .unwrap();

            assert!(m2.result.load(std::sync::atomic::Ordering::SeqCst));
        }
        _ => assert!(false, "Expected runtime to finish"),
    }
}
