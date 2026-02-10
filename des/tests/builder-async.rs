#![cfg(feature = "async")]

use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering},
    },
};

use des::{
    net::{ErrorKind, handlers::AsyncHandler},
    prelude::*,
};
use serial_test::serial;
use tokio::time::sleep;

#[test]
#[serial]
fn builder_async_fn_quasai_sync() {
    let done = Arc::new(AtomicBool::new(false));
    let d2 = done.clone();

    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(move |_| {
            let d2 = d2.clone();
            async move {
                d2.store(true, Ordering::SeqCst);
            }
        }),
    );

    assert_eq!(done.load(Ordering::SeqCst), false);
    let _ = Builder::seeded(123).build(sim.freeze()).run();
    assert_eq!(done.load(Ordering::SeqCst), true);
}

#[test]
#[serial]
fn builder_async_fn_sleep() {
    let time = Arc::new(AtomicU16::new(0));
    let t2 = time.clone();

    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(move |_| {
            let t2 = t2.clone();
            async move {
                sleep(Duration::from_secs(10)).await;
                t2.store(SimTime::now().as_secs() as u16, Ordering::SeqCst);
            }
        }),
    );

    assert_eq!(time.load(Ordering::SeqCst), 0);
    let _ = Builder::seeded(123).build(sim.freeze()).run();
    assert_eq!(time.load(Ordering::SeqCst), 10);
}

#[test]
#[serial]
fn builder_async_fn_message_recv() {
    let counter = Arc::new(AtomicU16::new(0));
    let c2 = counter.clone();

    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(move |mut rx| {
            let c2 = c2.clone();
            async move {
                while let Some(msg) = rx.recv().await {
                    c2.fetch_add(msg.header.id, Ordering::SeqCst);
                }
            }
        }),
    );
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim.freeze());
    rt.add_message_onto(gate.clone(), Message::default().with_id(1), 1.0.into());
    rt.add_message_onto(gate.clone(), Message::default().with_id(2), 2.0.into());
    rt.add_message_onto(gate.clone(), Message::default().with_id(3), 3.0.into());

    let _ = rt.run();
    assert_eq!(counter.load(Ordering::SeqCst), 6);
}

#[test]
#[serial]
fn builder_async_fn_channeled() {
    let counter = Arc::new(AtomicU16::new(0));
    let c2 = counter.clone();

    let mut sim = Sim::new(());
    sim.node(
        "tx",
        AsyncHandler::new(|_| async move {
            for i in 0..16 {
                sleep(Duration::from_secs(i)).await;
                let _ = send(Message::default().with_id(i as u16), "port");
            }
        }),
    );
    sim.node(
        "rx",
        AsyncHandler::new(move |mut rx| {
            let c2 = c2.clone();
            async move {
                while let Some(msg) = rx.recv().await {
                    c2.fetch_add(msg.header.id, Ordering::SeqCst);
                }
            }
        }),
    );

    let txg = sim.gate("tx", "port");
    let rxg = sim.gate("rx", "port");

    txg.connect_with(
        rxg,
        Some(DatarateChannel::new(DatarateChannelMetrics {
            bitrate: 10000,
            latency: Duration::from_millis(20),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::Queue(None),
        })),
    );

    let _ = Builder::seeded(123).build(sim.freeze()).run();
    assert_eq!(counter.load(Ordering::SeqCst), (0..16).sum());
}

#[test]
#[serial]
fn builder_async_failable() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::failable(|_| async move {
            if false {
                return Err(io::Error::new(io::ErrorKind::Other, "other"));
            }

            Ok(())
        }),
    );
    let _ = Builder::new().build(sim.freeze()).run();
}

#[test]
#[serial]
fn builder_async_failable_with_fail() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::failable(|_| async move {
            if true {
                return Err(io::Error::new(io::ErrorKind::Other, "other"));
            }

            Ok(())
        }),
    );
    let v = Builder::new().build(sim.freeze()).run();
    assert!(matches!(v.error.unwrap()[0].kind, ErrorKind::JoinError(_)));
}

#[test]
#[serial]
fn builder_async_no_join() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(|_| async move { std::future::pending().await }),
    );

    let _ = Builder::seeded(123).build(sim.freeze()).run();
}

#[test]
#[serial]
fn builder_async_require_join() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::io(|_| async move { std::future::pending().await }).require_join(),
    );

    let v = Builder::seeded(123).build(sim.freeze()).run();
    assert!(matches!(v.error.unwrap()[0].kind, ErrorKind::JoinError(_)));
}

#[test]
#[serial]
fn builder_async_restart() {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let mut sim = Sim::new(());
    let software = AsyncHandler::io(|_| async move {
        COUNTER.fetch_add(1, Ordering::SeqCst);

        des::time::sleep(Duration::from_secs(10)).await;
        current().shutdow_and_restart_in(Duration::from_secs(5));
        std::future::pending().await
    });
    assert_eq!(format!("{software:?}"), "AsyncFn");

    sim.node("alice", software);

    let _ = Builder::seeded(123)
        .max_time(25.0.into())
        .build(sim.freeze())
        .run();

    // once at 0, 15, next would be 30
    assert_eq!(COUNTER.load(Ordering::SeqCst), 2);
}
