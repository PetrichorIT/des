#![cfg(feature = "async")]
#![allow(unused_variables)]

use std::sync::{
    atomic::{AtomicBool, AtomicUsize},
    Arc,
};
use des::{prelude::*, time::sleep};
use tokio::{
    sync::{
        mpsc::{channel, Sender},
        Semaphore,
    },
    task::JoinHandle,
};

use serial_test::serial;

#[macro_use]
mod common;

// # Test case
// The module behaves like a sync module, not creating any more
// futures than the async call itself.

struct QuasaiSyncModule {
    counter: usize,
}
impl_build_named!(QuasaiSyncModule);

impl AsyncModule for QuasaiSyncModule {
    fn new() -> Self {
        Self { counter: 0 }
    }

    async fn handle_message(&mut self, msg: Message) {
        println!("[{}] Received msg: {}", module_name(), msg.header().id);
        self.counter += msg.header().id as usize;
    }
}

#[test]
#[serial]
fn quasai_sync_non_blocking() {
    let mut rt = NetworkApplication::new(());

    let module = QuasaiSyncModule::build_named(ObjectPath::from("RootModule".to_string()), &mut rt);

    let gate_a = module.create_gate("in", GateServiceType::Input);
    rt.register_module(module);

    let module_b =
        QuasaiSyncModule::build_named(ObjectPath::from("OtherRootModule".to_string()), &mut rt);

    let gate_b = module_b.create_gate("in", GateServiceType::Input);
    rt.register_module(module_b);

    let mut rt = Builder::seeded(123).build(rt);

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
            profiler,
        } => {
            assert_eq!(time, SimTime::ZERO);
            assert_eq!(profiler.event_count, 10);
        }
        _ => panic!("Expected runtime to finish"),
    }
}

// # Test case
// A module has 3 permantent tasks that each forward
// the message, the final one incrementing a module bound
// tracker
// The tasks shutdown with a shutdown message

struct MutipleTasksModule {
    handles: Vec<JoinHandle<()>>,
    sender: Option<Sender<Message>>,
    result: Arc<AtomicUsize>,
}
impl_build_named!(MutipleTasksModule);

impl AsyncModule for MutipleTasksModule {
    fn new() -> Self {
        Self {
            handles: Vec::new(),
            sender: None,
            result: Arc::new(AtomicUsize::new(0)),
        }
    }

    async fn at_sim_start(&mut self, _: usize) {
        let (txa, mut rxa) = channel::<Message>(8);
        let (txb, mut rxb) = channel(8);
        let (txc, mut rxc) = channel(8);

        let result = self.result.clone();

        let ta = tokio::spawn(async move {
            while let Some(v) = rxa.recv().await {
                let k = v.header().kind;
                txb.send(v).await.unwrap();

                if k == 42 {
                    rxa.close();
                    txb.closed().await;
                }
            }
        });

        let tb = tokio::spawn(async move {
            while let Some(v) = rxb.recv().await {
                let k = v.header().kind;
                txc.send(v).await.unwrap();

                if k == 42 {
                    rxb.close();
                    txc.closed().await;
                }
            }
        });

        let tc = tokio::spawn(async move {
            while let Some(v) = rxc.recv().await {
                let k = v.header().kind;
                result.fetch_add(v.header().id as usize, std::sync::atomic::Ordering::SeqCst);

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
    let mut rt = NetworkApplication::new(());

    let module_a =
        MutipleTasksModule::build_named(ObjectPath::from("RootModule".to_string()), &mut rt);

    let gate_a = module_a.create_gate("in", GateServiceType::Input);
    rt.register_module(module_a);

    let mut rt = Builder::seeded(123).build(rt);

    rt.add_message_onto(gate_a.clone(), Message::new().id(1).build(), SimTime::ZERO);
    rt.add_message_onto(gate_a, Message::new().id(2).build(), SimTime::ZERO);

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            profiler,
        } => {
            assert_eq!(time, SimTime::ZERO);

            //  2 * (Gate + HandleMessage)
            assert_eq!(profiler.event_count, 4);

            // let m1 = app
            //     .module(|m| m.module_core().name() == "RootModule")
            //     .unwrap()
            //     .self_as::<MutipleTasksModule>()
            //     .unwrap();

            // assert_eq!(m1.result.load(std::sync::atomic::Ordering::SeqCst), 100 + 3);
        }
        _ => panic!("Expected runtime to finish"),
    }
}

// # Test case
// A module sleeps upon receiving a message,
// This sleeps do NOT interfere with recv()

struct TimeSleepModule {
    counter: usize,
}
impl_build_named!(TimeSleepModule);

impl AsyncModule for TimeSleepModule {
    fn new() -> Self {
        Self { counter: 0 }
    }

    async fn handle_message(&mut self, msg: Message) {
        tracing::debug!("recv msg: {}", msg.str());
        let wait_time = msg.header().kind as u64;
        tracing::info!("<{}> [{}] Waiting for timer", module_name(), SimTime::now());
        sleep(Duration::from_secs(wait_time)).await;
        tracing::info!(
            "<{}> [{}] Done waiting for id: {}",
            module_name(),
            SimTime::now(),
            msg.header().id
        );
        self.counter += msg.header().id as usize
    }
}

#[test]
#[serial]
fn one_module_timers() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = NetworkApplication::new(());

    let module_a =
        TimeSleepModule::build_named(ObjectPath::from("RootModule".to_string()), &mut rt);

    let gate_a = module_a.create_gate("in", GateServiceType::Input);
    rt.register_module(module_a);

    let mut rt = Builder::seeded(123).build(rt);

    rt.add_message_onto(
        gate_a.clone(),
        Message::new().id(1).kind(1).build(),
        SimTime::ZERO,
    );
    rt.add_message_onto(
        gate_a,
        Message::new().id(2).kind(2).build(),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            profiler,
        } => {
            assert_eq!(time, 4.0);
            assert_eq!(profiler.event_count, 6);
        }
        _ => panic!("Expected runtime to finish"),
    }
}

// # Test case
// The module sleeps on message receival
// The sleeps should delay the next recv.

#[test]
#[serial]
fn one_module_delayed_recv() {
    let mut rt = NetworkApplication::new(());

    let module_a =
        TimeSleepModule::build_named(ObjectPath::from("RootModule".to_string()), &mut rt);

    let gate_a = module_a.create_gate("in", GateServiceType::Input);
    rt.register_module(module_a);

    let mut rt = Builder::seeded(123).build(rt);

    rt.add_message_onto(
        gate_a.clone(),
        Message::new().id(1).kind(2).build(),
        SimTime::ZERO,
    );
    rt.add_message_onto(
        gate_a,
        Message::new().id(2).kind(2).build(),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            profiler,
        } => {
            assert_eq!(time, 4.0);

            // 1) Gate #1 (0s)
            // 2) HandleMessage #1 (0s)
            // 3) Gate #2 (2s)
            // 4) HandleMessage #2 (2s) (will finish sleep but wakeup was added later)
            // 5) Wakeup aka NOP (2s)
            // 6) Wakeup - sleep reloved - send in '5 (4s)
            assert_eq!(profiler.event_count, 6);

            // let m1 = app
            //     .module(|m| m.module_core().name() == "RootModule")
            //     .unwrap()
            //     .self_as::<TimeSleepModule>()
            //     .unwrap();

            // assert_eq!(m1.counter, 3);
        }
        _ => panic!("Expected runtime to finish"),
    }
}

// # Test case
// Mutiple Modules delay themself with sleeps

#[test]
#[serial]
fn mutiple_module_delayed_recv() {
    let mut rt = NetworkApplication::new(());

    let module_a =
        TimeSleepModule::build_named(ObjectPath::from("RootModule".to_string()), &mut rt);
    let gate_a = module_a.create_gate("in", GateServiceType::Input);
    rt.register_module(module_a);

    let module_b =
        TimeSleepModule::build_named(ObjectPath::from("OtherRootModule".to_string()), &mut rt);
    let gate_b = module_b.create_gate("in", GateServiceType::Input);
    rt.register_module(module_b);

    let mut rt = Builder::seeded(123).build(rt);

    // # Module 1
    //  |0  |1  |2  |3  |4  |5  |6
    //       <ID=1_>
    //          ....<ID=2_>
    rt.add_message_onto(
        gate_a.clone(),
        Message::new().id(1).kind(2).build(),
        SimTime::from_duration(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_a,
        Message::new().id(2).kind(2).build(),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    // # Module 1
    //  |0  |1  |2  |3  |4  |5  |6
    //      <ID>
    //          <ID=20>
    rt.add_message_onto(
        gate_b.clone(),
        Message::new().id(10).kind(1).build(),
        SimTime::from_duration(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_b,
        Message::new().id(20).kind(2).build(),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            profiler,
        } => {
            assert_eq!(time, 5.0);

            assert_eq!(profiler.event_count, 12);

            // let m1 = app
            //     .module(|m| m.module_core().name() == "RootModule")
            //     .unwrap()
            //     .self_as::<TimeSleepModule>()
            //     .unwrap();

            // assert_eq!(m1.counter, 3);

            // let m2 = app
            //     .module(|m| m.module_core().name() == "OtherRootModule")
            //     .unwrap()
            //     .self_as::<TimeSleepModule>()
            //     .unwrap();

            // assert_eq!(m2.counter, 30);
        }
        _ => panic!("Expected runtime to finish"),
    }
}

struct SemaphoreModule {
    semaphore: Arc<Semaphore>,
    handle: Option<JoinHandle<()>>,
    result: Arc<AtomicBool>,
}
impl_build_named!(SemaphoreModule);

impl AsyncModule for SemaphoreModule {
    fn new() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(0)),
            handle: None,
            result: Arc::new(AtomicBool::new(false)),
        }
    }

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
        self.semaphore.add_permits(msg.header().kind as usize);
    }
}

#[test]
#[serial]
fn semaphore_in_waiting_task() {
    let mut rt = NetworkApplication::new(());

    let module_a =
        SemaphoreModule::build_named(ObjectPath::from("RootModule".to_string()), &mut rt);
    let gate_a = module_a.create_gate("in", GateServiceType::Input);
    rt.register_module(module_a);

    let module_b =
        SemaphoreModule::build_named(ObjectPath::from("OtherRootModule".to_string()), &mut rt);
    let gate_b = module_b.create_gate("in", GateServiceType::Input);
    rt.register_module(module_b);

    let mut rt = Builder::seeded(123).build(rt);

    rt.add_message_onto(
        gate_a.clone(),
        Message::new().id(1).kind(2).build(),
        SimTime::from_duration(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_a,
        Message::new().id(2).kind(3).build(),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    rt.add_message_onto(
        gate_b.clone(),
        Message::new().id(10).kind(2).build(),
        SimTime::from_duration(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_b.clone(),
        Message::new().id(20).kind(2).build(),
        SimTime::from_duration(Duration::new(2, 0)),
    );
    rt.add_message_onto(
        gate_b,
        Message::new().id(20).kind(1).build(),
        SimTime::from_duration(Duration::new(3, 0)),
    );

    let result = rt.run();
    match result {
        RuntimeResult::Finished {
            app,
            time,
            profiler,
        } => {
            assert_eq!(time, 3.0);
            assert_eq!(profiler.event_count, 10);
        }
        _ => panic!("Expected runtime to finish"),
    }
}

struct ShouldBlockSimStart {}
impl_build_named!(ShouldBlockSimStart);


impl AsyncModule for ShouldBlockSimStart {
    fn new() -> Self {
        Self {}
    }

    async fn handle_message(&mut self, _: Message) {}

    async fn at_sim_start(&mut self, _: usize) {
        let sem = Semaphore::new(0);
        let _ = sem.acquire().await.expect("CRASH");
    }
}

// #[test]
// #[should_panic = "Join Idle: RuntimeIdle(())"]
// fn sim_start_deadlock() {
//     let mut rt = NetworkApplication::new(());
//     let mut cx = BuildContext::new(&mut rt);

//     let module_a = ShouldBlockSimStart::build_named(
//         ObjectPath::from("RootModule".to_string()),
//         &mut cx,
//     );

//     cx.create_module(module_a);

//     let rt = Runtime::new(rt);

//     let _result = rt.run();
// }
