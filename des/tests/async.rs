#![cfg(feature = "async")]
#![allow(unused_variables)]

use des::{prelude::*, time::sleep};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize},
    Arc,
};
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

#[derive(Default)]
struct QuasaiSyncModule {
    counter: usize,
}

impl AsyncModule for QuasaiSyncModule {
    async fn handle_message(&mut self, msg: Message) {
        println!("[{}] Received msg: {}", current().name(), msg.header().id);
        self.counter += msg.header().id as usize;
    }
}

#[test]
#[serial]
fn quasai_sync_non_blocking() {
    let mut rt = Sim::new(());
    rt.node("root", QuasaiSyncModule::default());
    rt.node("other", QuasaiSyncModule::default());

    let gate_a = rt.gate("root", "a");
    let gate_b = rt.gate("other", "b");

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

#[derive(Default)]
struct MutipleTasksModule {
    handles: Vec<JoinHandle<()>>,
    sender: Option<Sender<Message>>,
    result: Arc<AtomicUsize>,
}

impl AsyncModule for MutipleTasksModule {
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
    let mut rt = Sim::new(());
    rt.node("root", MutipleTasksModule::default());

    let gate_a = rt.gate("root", "in");

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

#[derive(Default)]
struct TimeSleepModule {
    counter: usize,
}

impl AsyncModule for TimeSleepModule {
    async fn handle_message(&mut self, msg: Message) {
        tracing::debug!("recv msg: {}", msg.str());
        let wait_time = msg.header().kind as u64;
        tracing::info!(
            "<{}> [{}] Waiting for timer",
            current().name(),
            SimTime::now()
        );
        sleep(Duration::from_secs(wait_time)).await;
        tracing::info!(
            "<{}> [{}] Done waiting for id: {}",
            current().name(),
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

    let mut rt = Sim::new(());
    rt.node("root", TimeSleepModule::default());

    let gate_a = rt.gate("root", "a");

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
    let mut rt = Sim::new(());
    rt.node("root", TimeSleepModule::default());

    let gate_a = rt.gate("root", "in");

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
    let mut rt = Sim::new(());
    rt.node("a", TimeSleepModule::default());
    rt.node("b", TimeSleepModule::default());

    let gate_a = rt.gate("a", "in");
    let gate_b = rt.gate("b", "in");

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

impl Default for SemaphoreModule {
    fn default() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(0)),
            handle: None,
            result: Arc::new(AtomicBool::new(false)),
        }
    }
}

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
        self.semaphore.add_permits(msg.header().kind as usize);
    }
}

#[test]
#[serial]
fn semaphore_in_waiting_task() {
    let mut rt = Sim::new(());
    rt.node("a", SemaphoreModule::default());
    rt.node("b", SemaphoreModule::default());

    let gate_a = rt.gate("a", "in");
    let gate_b = rt.gate("b", "in");

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
