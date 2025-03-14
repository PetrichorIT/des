#![cfg(feature = "async")]
#![allow(unused_variables)]

use des::{
    net::{
        module::{join, Module},
        AsyncFn, JoinError,
    },
    prelude::*,
    runtime::RuntimeError,
    time::{self, sleep, timeout, timeout_at, MissedTickBehavior},
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use tokio::{
    sync::{
        mpsc::{self, channel, Sender},
        Semaphore,
    },
    task::{JoinHandle, JoinSet},
};

use serial_test::serial;

// # Test case
// The module behaves like a sync module, not creating any more
// futures than the async call itself.

#[derive(Default)]
struct QuasaiSyncModule {
    counter: usize,
}

impl Module for QuasaiSyncModule {
    fn handle_message(&mut self, msg: Message) {
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
        Ok((app, time, profiler)) => {
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
    handles: JoinSet<()>,
    sender: Option<Sender<Message>>,
    result: Arc<AtomicUsize>,
}

impl Module for MutipleTasksModule {
    fn at_sim_start(&mut self, _: usize) {
        let (txa, mut rxa) = channel::<Message>(8);
        let (txb, mut rxb) = channel(8);
        let (txc, mut rxc) = channel(8);

        let result = self.result.clone();

        self.handles.spawn(async move {
            while let Some(v) = rxa.recv().await {
                let k = v.header().kind;
                txb.send(v).await.unwrap();

                if k == 42 {
                    rxa.close();
                    txb.closed().await;
                }
            }
        });

        self.handles.spawn(async move {
            while let Some(v) = rxb.recv().await {
                let k = v.header().kind;
                txc.send(v).await.unwrap();

                if k == 42 {
                    rxb.close();
                    txc.closed().await;
                }
            }
        });

        self.handles.spawn(async move {
            while let Some(v) = rxc.recv().await {
                let k = v.header().kind;
                result.fetch_add(v.header().id as usize, std::sync::atomic::Ordering::SeqCst);

                if k == 42 {
                    rxc.close();
                }
            }
        });

        self.sender = Some(txa);
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        for i in 0..self.handles.len() {
            assert!(
                self.handles.try_join_next().is_some(),
                "Failed to join {i}-th handle"
            );
        }
        Ok(())
    }

    fn handle_message(&mut self, msg: Message) {
        self.sender.as_ref().unwrap().try_send(msg).unwrap()
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
    rt.add_message_onto(gate_a.clone(), Message::new().id(2).build(), SimTime::ZERO);
    rt.add_message_onto(gate_a, Message::new().kind(42).build(), SimTime::ZERO);

    let result = rt.run();
    match result {
        Ok((app, time, profiler)) => {
            assert_eq!(time, SimTime::ZERO);

            //  3 * (Gate + HandleMessage)
            assert_eq!(profiler.event_count, 6);

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
struct TimeSleepModule {}

impl Module for TimeSleepModule {
    fn handle_message(&mut self, msg: Message) {
        tokio::spawn(async move {
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
        });
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
        Ok((app, time, profiler)) => {
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
        Ok((app, time, profiler)) => {
            assert_eq!(time, 4.0);

            // 1) Gate #1 (0s)
            // 2) HandleMessage #1 (0s)
            // 3) Gate #2 (2s)
            // 4) HandleMessage #2 (2s) (will finish sleep but wakeup was added later)
            // 5) Wakeup aka NOP (2s)
            // 6) Wakeup - sleep reloved - send in '5 (4s)
            assert_eq!(profiler.event_count, 6);
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
        Ok((app, time, profiler)) => {
            assert_eq!(time, 4.0); // parallel exec is possible
            assert_eq!(profiler.event_count, 12);
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

impl Module for SemaphoreModule {
    fn at_sim_start(&mut self, _: usize) {
        let sem = self.semaphore.clone();
        let res = self.result.clone();
        self.handle = Some(tokio::spawn(async move {
            let premit = sem.acquire_many(5).await.unwrap();
            println!("[{}] Aquired semaphore", SimTime::now());
            res.fetch_or(true, std::sync::atomic::Ordering::SeqCst);
            drop(premit)
        }));
    }

    fn handle_message(&mut self, msg: Message) {
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
        Ok((app, time, profiler)) => {
            assert_eq!(time, 3.0);
            assert_eq!(profiler.event_count, 10);
        }
        _ => panic!("Expected runtime to finish"),
    }
}

#[test]
#[serial]
fn async_time_sleep_far_future() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncFn::new(|rx| async move {
            assert_eq!(SimTime::now(), 0.0);
            time::sleep_until(10.0.into()).await;
            assert_eq!(SimTime::now(), 10.0);
            let sleep = time::sleep(Duration::MAX);
            assert_eq!(sleep.deadline(), SimTime::MAX);
            assert!(!sleep.is_elapsed());

            sleep.await;
            panic!("should never be reached");
        }),
    );

    let result = Builder::seeded(123).build(sim).run();
    assert_eq!(result.unwrap().1, 10.0);
}

#[test]
#[serial]
fn async_time_sleep_select() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncFn::new(|rx| async move {
            tokio::select! {
                _ = time::sleep(Duration::from_secs(10)) => unreachable!(),
                _ = time::sleep(Duration::from_secs(5)) => println!("resolved"),
            }
        })
        .require_join(),
    );

    let result = Builder::seeded(123).build(sim).run().unwrap();
    assert_eq!(result.1, 5.0);
    assert_eq!(result.2.event_count, 1); // Just async wakeup for 5s, 10s will never be scheduled
}

#[test]
#[serial]
fn async_time_sleep_reset() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncFn::new(|rx| async move {
            let sleep = time::sleep(Duration::from_secs(5));
            tokio::pin!(sleep);

            sleep.as_mut().reset(10.0.into());
            sleep.await
        })
        .require_join(),
    );

    let result = Builder::seeded(123).build(sim).run().unwrap();
    assert_eq!(result.1, 10.0);
    assert_eq!(result.2.event_count, 1); // Just async wakeup for 10s, 5s was not yet scheduled
}

#[test]
#[serial]
fn async_time_timeout() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncFn::new(|rx| async move {
            let result: Result<i32, time::error::Elapsed> =
                timeout(Duration::from_secs(10), std::future::pending()).await;
            assert!(result.is_err());

            let (tx, mut rx) = mpsc::channel(1);

            let handle = tokio::task::spawn(async move {
                time::sleep(Duration::from_secs(5)).await;
                tx.send(42).await.unwrap();
                println!("1:{}", SimTime::now());
            });

            println!("0:{}", SimTime::now());
            let result: Result<Option<i32>, time::error::Elapsed> =
                timeout_at(42.0.into(), rx.recv()).await;
            assert_eq!(result, Ok(Some(42)));

            println!("2: {}", SimTime::now());
            handle.await.unwrap();
            println!("3: {}", SimTime::now());
        })
        .require_join(),
    );

    let result = Builder::seeded(123).build(sim).run().unwrap();
    assert_eq!(result.1, 15.0);
    // why 15s?
    // wakeup 20s will never be scheduled, since
    // -> wakeup 15s from tokio::task is allready scheduled
    // -> upon completion of 15s timeout 20s is allready removed
}

#[test]
#[serial]
fn async_time_timeout_far_future() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncFn::new(|rx| async move {
            // add a sleep to get a nonempty sim
            time::sleep(Duration::from_secs(42)).await;

            let mut timeout = timeout(Duration::MAX, std::future::pending());
            let _: &std::future::Pending<i32> = timeout.get_ref();
            let _: &mut std::future::Pending<i32> = timeout.get_mut();

            let result: Result<i32, time::error::Elapsed> = timeout.await;
            panic!("will never be reached")
        }),
    );

    let result = Builder::seeded(123).build(sim).run().unwrap();
    assert_eq!(result.1, 42.0);
}

#[test]
#[serial]
fn async_time_interval() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncFn::new(|rx| async move {
            // (0) No missed ticks
            let counter = Arc::new(AtomicUsize::new(0));

            let c = counter.clone();
            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(1));
                assert_eq!(interval.period(), Duration::from_secs(1));
                assert_eq!(
                    interval.missed_tick_behavior(),
                    MissedTickBehavior::default()
                );

                loop {
                    interval.tick().await;
                    c.fetch_add(1, Ordering::SeqCst);
                }
            });

            time::sleep(Duration::from_secs_f64(7.5)).await;
            assert_eq!(counter.load(Ordering::SeqCst), 1 + 7);
        }),
    );

    let _ = Builder::seeded(123).max_time(100.0.into()).build(sim).run();
}

#[test]
#[serial]
fn async_time_interval_missed_tick_behaviour() {
    let mut sim = Sim::new(());
    sim.node(
        "burst",
        AsyncFn::new(|rx| async move {
            // (0) No missed ticks
            let mut interval = time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(MissedTickBehavior::Burst);

            time::sleep(Duration::from_secs_f64(4.5)).await;

            for _ in 0..6 {
                // expected ticks at 0, 1, 2, 3, 4, 5
                // got at 4.5, ..., 4.5, 5
                interval.tick().await;
            }
            assert_eq!(SimTime::now(), 5.0);
        }),
    );

    sim.node(
        "delay",
        AsyncFn::new(|rx| async move {
            // (0) No missed ticks
            let mut interval = time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            let mut last = SimTime::now();

            time::sleep(Duration::from_secs_f64(4.5)).await;

            for i in 0..6 {
                // expected ticks at 0, 1, 2, 3, 4, 5
                // got at 4.5, ..., 4.5, 5
                interval.tick().await;
                if i != 0 {
                    assert_eq!(SimTime::now(), last + 1.0);
                }
                last = SimTime::now();
            }

            assert_eq!(SimTime::now(), 4.5 + 5.0);
        }),
    );

    sim.node(
        "skip",
        AsyncFn::new(|rx| async move {
            // (0) No missed ticks
            let mut interval = time::interval_at(0.0.into(), Duration::from_secs(1));
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

            time::sleep(Duration::from_secs_f64(4.5)).await;

            for i in 0..6 {
                // expected ticks at 0, 1, 2, 3, 4, 5
                // got at 4.5, ..., 4.5, 5
                interval.tick().await;
                if i != 0 {
                    assert_eq!(SimTime::now().subsec_millis(), 0);
                }
            }

            assert_eq!(SimTime::from(5.0).elapsed(), Duration::from_secs(4));
            assert_eq!(SimTime::now(), 9.0);
        }),
    );

    let _ = Builder::seeded(123).max_time(100.0.into()).build(sim).run();
}

struct JoinOnModule;
impl Module for JoinOnModule {
    fn at_sim_start(&mut self, _stage: usize) {
        join(tokio::spawn(async move {
            std::future::pending::<()>().await;
        }));
    }
}

#[test]
#[serial]
fn async_join_on_module_fail() {
    let mut sim = Sim::new(());
    sim.node("main", JoinOnModule);

    let v = Builder::seeded(123).build(sim).run();
    assert!(v.unwrap_err()[0]
        .as_any()
        .downcast_ref::<JoinError>()
        .is_some())
}

struct PanicIsJoinable;
impl Module for PanicIsJoinable {
    fn at_sim_start(&mut self, _stage: usize) {
        join(tokio::spawn(async move { panic!("Panic-Source") }));
    }
}

#[test]
#[serial]
fn async_join_paniced_will_join_but_fail() {
    let mut sim = Sim::new(());
    sim.node("main", PanicIsJoinable);

    let v = Builder::seeded(123).build(sim).run();
    assert!(v.unwrap_err()[0]
        .as_any()
        .downcast_ref::<JoinError>()
        .is_some())
}

struct SpawnButNeverJoin;
impl Module for SpawnButNeverJoin {
    fn at_sim_start(&mut self, _stage: usize) {
        join(tokio::spawn(async move {
            std::future::pending::<()>().await;
        }));
    }
}

#[test]
#[serial]
fn runtime_require_join() {
    let mut sim = Sim::new(());
    sim.node("main", SpawnButNeverJoin);

    let _ = Builder::seeded(123).build(sim).run();
}
