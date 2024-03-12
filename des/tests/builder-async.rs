#![cfg(feature = "async")]

use std::sync::{
    atomic::{AtomicBool, AtomicU16, Ordering},
    Arc,
};

use des::{net::AsyncFn, prelude::*, time::sleep};
use serial_test::serial;

#[test]
#[serial]
fn builder_async_fn_quasai_sync() {
    let done = Arc::new(AtomicBool::new(false));
    let d2 = done.clone();

    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncFn::new(move |_| {
            let d2 = d2.clone();
            async move {
                d2.store(true, Ordering::SeqCst);
            }
        }),
    );

    assert_eq!(done.load(Ordering::SeqCst), false);
    let _ = Builder::seeded(123).build(sim).run();
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
        AsyncFn::new(move |_| {
            let t2 = t2.clone();
            async move {
                sleep(Duration::from_secs(10)).await;
                t2.store(SimTime::now().as_secs() as u16, Ordering::SeqCst);
            }
        }),
    );

    assert_eq!(time.load(Ordering::SeqCst), 0);
    let _ = Builder::seeded(123).build(sim).run();
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
        AsyncFn::new(move |mut rx| {
            let c2 = c2.clone();
            async move {
                while let Some(msg) = rx.recv().await {
                    c2.fetch_add(msg.header().id, Ordering::SeqCst);
                }
            }
        }),
    );
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate.clone(), Message::new().id(1).build(), 1.0.into());
    rt.add_message_onto(gate.clone(), Message::new().id(2).build(), 2.0.into());
    rt.add_message_onto(gate.clone(), Message::new().id(3).build(), 3.0.into());

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
        AsyncFn::new(|_| async move {
            for i in 0..16 {
                sleep(Duration::from_secs(i)).await;
                send(Message::new().id(i as u16).build(), "port");
            }
        }),
    );
    sim.node(
        "rx",
        AsyncFn::new(move |mut rx| {
            let c2 = c2.clone();
            async move {
                while let Some(msg) = rx.recv().await {
                    c2.fetch_add(msg.header().id, Ordering::SeqCst);
                }
            }
        }),
    );

    let txg = sim.gate("tx", "port");
    let rxg = sim.gate("rx", "port");

    txg.connect(
        rxg,
        Some(Channel::new(ChannelMetrics {
            bitrate: 10000,
            latency: Duration::from_millis(20),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::Queue(None),
        })),
    );

    let _ = Builder::seeded(123).build(sim).run();
    assert_eq!(counter.load(Ordering::SeqCst), (0..16).sum());
}
