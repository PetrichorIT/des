#![cfg(feature = "async")]
#![allow(unused_variables)]

use des::{
    net::{
        Error, ErrorKind, Failure,
        handlers::AsyncHandler,
        module::{Module, UnwindBehaviour},
    },
    prelude::*,
    time::{self, MissedTickBehavior, sleep, timeout, timeout_at},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::{
    sync::{
        Semaphore,
        mpsc::{self, Sender, channel},
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
        println!("[{}] Received msg: {}", current().name(), msg.header.id);
        self.counter += msg.header.id as usize;
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

    let mut rt = Builder::seeded(123).build(rt.freeze());

    rt.add_message_onto(gate_a.clone(), Message::default().with_id(1), SimTime::ZERO);
    rt.add_message_onto(gate_a, Message::default().with_id(2), SimTime::ZERO);

    rt.add_message_onto(gate_b.clone(), Message::default().with_id(1), SimTime::ZERO);
    rt.add_message_onto(gate_b.clone(), Message::default().with_id(2), SimTime::ZERO);
    rt.add_message_onto(gate_b, Message::default().with_id(3), SimTime::ZERO);

    let result = rt.run();
    assert!(result.error.is_none());
    assert_eq!(result.time, SimTime::ZERO);
    assert_eq!(result.profiler.event_count, 12); // (+2 start signal)
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
                let k = v.header.kind;
                txb.send(v).await.unwrap();

                if k == 42 {
                    rxa.close();
                    txb.closed().await;
                }
            }
        });

        self.handles.spawn(async move {
            while let Some(v) = rxb.recv().await {
                let k = v.header.kind;
                txc.send(v).await.unwrap();

                if k == 42 {
                    rxb.close();
                    txc.closed().await;
                }
            }
        });

        self.handles.spawn(async move {
            while let Some(v) = rxc.recv().await {
                let k = v.header.kind;
                result.fetch_add(v.header.id as usize, std::sync::atomic::Ordering::SeqCst);

                if k == 42 {
                    rxc.close();
                }
            }
        });

        self.sender = Some(txa);
    }

    fn at_sim_end(&mut self) -> Result<(), Error> {
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

    let mut rt = Builder::seeded(123).build(rt.freeze());

    rt.add_message_onto(gate_a.clone(), Message::default().with_id(1), SimTime::ZERO);
    rt.add_message_onto(gate_a.clone(), Message::default().with_id(2), SimTime::ZERO);
    rt.add_message_onto(gate_a, Message::default().with_kind(42), SimTime::ZERO);

    let result = rt.run();
    assert!(result.error.is_none());
    assert_eq!(result.time, SimTime::ZERO);
    //  3 * (Gate + HandleMessage) (+1 start signal)
    assert_eq!(result.profiler.event_count, 7);
}

// # Test case
// A module sleeps upon receiving a message,
// This sleeps do NOT interfere with recv()

#[derive(Default)]
struct TimeSleepModule {}

impl Module for TimeSleepModule {
    fn handle_message(&mut self, msg: Message) {
        tokio::spawn(async move {
            tracing::debug!("recv msg: {msg}");
            let wait_time = msg.header.kind as u64;
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
                msg.header.id
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

    let mut rt = Builder::seeded(123).build(rt.freeze());

    rt.add_message_onto(
        gate_a.clone(),
        Message::default().with_id(1).with_kind(1),
        SimTime::ZERO,
    );
    rt.add_message_onto(
        gate_a,
        Message::default().with_id(2).with_kind(2),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    let result = rt.run();

    assert!(result.error.is_none());
    assert_eq!(result.time, 4.0);
    assert_eq!(result.profiler.event_count, 7); // (+1 start signal)
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

    let mut rt = Builder::seeded(123).build(rt.freeze());

    rt.add_message_onto(
        gate_a.clone(),
        Message::default().with_id(1).with_kind(2),
        SimTime::ZERO,
    );
    rt.add_message_onto(
        gate_a,
        Message::default().with_id(2).with_kind(2),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    let result = rt.run();
    assert!(result.error.is_none());
    assert_eq!(result.time, 4.0);
    assert_eq!(result.profiler.event_count, 7); // (+2 start signal)
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

    let mut rt = Builder::seeded(123).build(rt.freeze());

    // # Module 1
    //  |0  |1  |2  |3  |4  |5  |6
    //       <ID=1_>
    //          ....<ID=2_>
    rt.add_message_onto(
        gate_a.clone(),
        Message::default().with_id(1).with_kind(2),
        SimTime::from_duration(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_a,
        Message::default().with_id(2).with_kind(2),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    // # Module 1
    //  |0  |1  |2  |3  |4  |5  |6
    //      <ID>
    //          <ID=20>
    rt.add_message_onto(
        gate_b.clone(),
        Message::default().with_id(10).with_kind(1),
        SimTime::from_duration(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_b,
        Message::default().with_id(20).with_kind(2),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    let result = rt.run();
    assert!(result.error.is_none());
    assert_eq!(result.time, 4.0);
    assert_eq!(result.profiler.event_count, 14); // (+2 start signal)
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
        self.semaphore.add_permits(msg.header.kind as usize);
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

    let mut rt = Builder::seeded(123).build(rt.freeze());

    rt.add_message_onto(
        gate_a.clone(),
        Message::default().with_id(1).with_kind(2),
        SimTime::from_duration(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_a,
        Message::default().with_id(2).with_kind(3),
        SimTime::from_duration(Duration::new(2, 0)),
    );

    rt.add_message_onto(
        gate_b.clone(),
        Message::default().with_id(10).with_kind(2),
        SimTime::from_duration(Duration::new(1, 0)),
    );
    rt.add_message_onto(
        gate_b.clone(),
        Message::default().with_id(20).with_kind(2),
        SimTime::from_duration(Duration::new(2, 0)),
    );
    rt.add_message_onto(
        gate_b,
        Message::default().with_id(20).with_kind(1),
        SimTime::from_duration(Duration::new(3, 0)),
    );

    let result = rt.run();
    assert!(result.error.is_none());
    assert_eq!(result.time, 3.0);
    assert_eq!(result.profiler.event_count, 12); // (+2 start signal)
}

#[test]
#[serial]
fn async_time_sleep_far_future() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(|rx| async move {
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

    let result = Builder::seeded(123).build(sim.freeze()).run();
    assert!(result.error.is_none());
    assert_eq!(result.time, 10.0);
}

#[test]
#[serial]
fn async_time_sleep_select() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(|rx| async move {
            tokio::select! {
                _ = time::sleep(Duration::from_secs(10)) => unreachable!(),
                _ = time::sleep(Duration::from_secs(5)) => println!("resolved"),
            }
        })
        .require_join(),
    );

    let result = Builder::seeded(123).build(sim.freeze()).run();
    assert!(result.error.is_none());
    assert_eq!(result.time, 5.0);
    assert_eq!(result.profiler.event_count, 2); // Just async wakeup for 5s, 10s will never be scheduled (+1 start signal)
}

#[test]
#[serial]
fn async_time_sleep_reset() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(|rx| async move {
            let sleep = time::sleep(Duration::from_secs(5));
            tokio::pin!(sleep);

            sleep.as_mut().reset(10.0.into());
            sleep.await
        })
        .require_join(),
    );

    let result = Builder::seeded(123).build(sim.freeze()).run();
    assert!(result.error.is_none());
    assert_eq!(result.time, 10.0);
    assert_eq!(result.profiler.event_count, 2); // Just async wakeup for 10s, 5s was not yet scheduled (+1 start signal)
}

#[test]
#[serial]
fn async_time_timeout() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(|rx| async move {
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

    let result = Builder::seeded(123).build(sim.freeze()).run();
    assert!(result.error.is_none());
    assert_eq!(result.time, 15.0);
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
        AsyncHandler::new(|rx| async move {
            // add a sleep to get a nonempty sim
            time::sleep(Duration::from_secs(42)).await;

            let mut timeout = timeout(Duration::MAX, std::future::pending());
            let _: &std::future::Pending<i32> = timeout.get_ref();
            let _: &mut std::future::Pending<i32> = timeout.get_mut();

            let result: Result<i32, time::error::Elapsed> = timeout.await;
            panic!("will never be reached")
        }),
    );

    let result = Builder::seeded(123).build(sim.freeze()).run();
    assert!(result.error.is_none());
    assert_eq!(result.time, 42.0);
}

#[test]
#[serial]
fn async_time_interval() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(|rx| async move {
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

    let _ = Builder::seeded(123)
        .max_time(100.0.into())
        .build(sim.freeze())
        .run();
}

#[test]
#[serial]
fn async_time_interval_missed_tick_behaviour() {
    let mut sim = Sim::new(());
    sim.node(
        "burst",
        AsyncHandler::new(|rx| async move {
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
        AsyncHandler::new(|rx| async move {
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
        AsyncHandler::new(|rx| async move {
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

    let _ = Builder::seeded(123)
        .max_time(100.0.into())
        .build(sim.freeze())
        .run();
}

struct JoinOnModule;
impl Module for JoinOnModule {
    fn at_sim_start(&mut self, _stage: usize) {
        current().join(tokio::spawn(async move {
            std::future::pending::<()>().await;
        }));
    }
}

#[test]
#[serial]
fn async_join_on_module_fail() {
    let mut sim = Sim::new(());
    sim.node("main", JoinOnModule);

    let v = Builder::seeded(123).build(sim.freeze()).run();
    assert!(matches!(v.error.unwrap()[0].kind, ErrorKind::JoinError(_)));
}

struct PanicIsJoinable;
impl Module for PanicIsJoinable {
    fn at_sim_start(&mut self, _stage: usize) {
        current().join(tokio::spawn(async move { panic!("Panic-Source") }));
    }
}

#[test]
#[serial]
fn async_join_paniced_will_join_but_fail() {
    let mut sim = Sim::new(());
    sim.node("main", PanicIsJoinable);

    let v = Builder::seeded(123).build(sim.freeze()).run();
    assert!(matches!(v.error.unwrap()[0].kind, ErrorKind::JoinError(_)));
}

struct SpawnButNeverJoin;
impl Module for SpawnButNeverJoin {
    fn at_sim_start(&mut self, _stage: usize) {
        current().join(tokio::spawn(async move {
            std::future::pending::<()>().await;
        }));
    }
}

#[test]
#[serial]
fn runtime_require_join() {
    let mut sim = Sim::new(());
    sim.node("main", SpawnButNeverJoin);

    let _ = Builder::seeded(123).build(sim.freeze()).run();
}

#[test]
#[serial]
fn wait_for_sim_start_fin() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    let (tx, rx) = std::sync::mpsc::channel();

    for i in 0..3 {
        let tx1 = tx.clone();
        sim.node(
            format!("alice-{i}"),
            AsyncHandler::new(move |_| {
                let tx1 = tx1.clone();
                async move {
                    tx1.send((i, "pre")).unwrap();
                    current().wait_for_start().await;
                    tx1.send((i, "post")).unwrap();
                }
            }),
        );
    }

    let _ = Builder::seeded(123).build(sim.freeze()).run().as_result()?;

    let mut buf = Vec::new();
    while let Ok(v) = rx.try_recv() {
        buf.push(v);
    }

    assert_eq!(
        buf,
        &[
            (0, "pre"),
            (1, "pre"),
            (2, "pre"),
            (0, "post"),
            (1, "post"),
            (2, "post"),
        ]
    );

    Ok(())
}

#[test]
#[serial]
fn panic_stops_sim_immediately() -> Result<(), Error> {
    let mut sim = Sim::new(());
    let cfg = UnwindBehaviour {
        on_panic_catch: false,
        ..Default::default()
    };
    sim.set_default_unwind_behavior(cfg);
    sim.node(
        "alice",
        AsyncHandler::once(move |_| async move {
            assert_eq!(current().unwind_behaviour(), cfg);
            sleep(Duration::from_secs(1)).await;
            panic!("Huh something went wrong");
        })
        .require_join(),
    );

    let counter = Arc::new(AtomicUsize::new(0));
    let counter2 = counter.clone();
    sim.node(
        "bob",
        AsyncHandler::once(move |_| async move {
            loop {
                sleep(Duration::from_secs_f64(0.4)).await;
                counter.fetch_add(1, Ordering::SeqCst);
                schedule_in(Message::default(), Duration::from_secs(1));
            }
        }),
    );

    let _err = Builder::seeded(123)
        .max_time(10.0.into())
        .build(sim.freeze())
        .run()
        .error
        .unwrap();

    assert_eq!(counter2.load(Ordering::SeqCst), 2); // at 0.4, 0.8

    Ok(())
}
