#![cfg(feature = "async")]

use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
    Arc,
};

use des::{
    net::{AsyncBuilder, NodeCfg},
    prelude::*,
    time::sleep,
};

#[test]
#[serial_test::serial]
fn builder_single_module() {
    let done = Arc::new(AtomicBool::new(false));
    let done_c = done.clone();

    let mut sim = AsyncBuilder::new();
    sim.node("single", move |_| {
        let done_c = done_c.clone();
        async move {
            done_c.store(true, SeqCst);
            Ok(())
        }
    });

    let rt = Builder::seeded(123).build(sim.build());
    let _ = rt.run();
    assert_eq!(done.load(SeqCst), true);
}

#[test]
#[serial_test::serial]
fn builder_connected_modules() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_c = counter.clone();

    let mut sim = AsyncBuilder::new();

    sim.node("sender", move |_| async move {
        println!("sender start");
        for _ in 0..10 {
            send(Message::new().build(), ("port", 0));
            sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    });
    sim.node("receiver", move |mut rx| {
        let counter = counter.clone();
        async move {
            while let Some(_msg) = rx.recv().await {
                counter.fetch_add(1, SeqCst);
            }
            Ok(())
        }
    });

    sim.connect("sender", "receiver");

    println!("init");
    let rt = Builder::seeded(123).build(sim.build());
    let _ = rt.run();
    assert_eq!(counter_c.load(SeqCst), 10);
}

#[test]
#[serial_test::serial]
fn builder_delayed_links() {
    let mut sim = AsyncBuilder::new();
    sim.set_default_cfg(NodeCfg { join: true });

    sim.node("client", |_| async {
        send(Message::new().build(), ("port", 0));
        Ok(())
    });
    sim.node("server", |mut rx| async move {
        let _ = rx.recv().await;
        assert_ne!(SimTime::now(), SimTime::ZERO);
        Ok(())
    });
    sim.connect_with(
        "client",
        "server",
        Some(ChannelMetrics {
            bitrate: 10000,
            latency: Duration::from_millis(10),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::default(),
        }),
    );

    let _ = Builder::seeded(123).build(sim.build()).run();
}

#[test]
#[serial_test::serial]
fn builder_recognize_parents() {
    let mut sim = AsyncBuilder::new();
    sim.set_default_cfg(NodeCfg { join: true });
    sim.node("parent", |_| async {
        let ch = current().child("child").expect("parent sees no child");
        assert_eq!(ch.name(), "child");
        Ok(())
    });
    sim.node_with_parent("child", "parent", |_| async {
        let p = current().parent().expect("child sees no parent");
        assert_eq!(p.name(), "parent");
        Ok(())
    });

    let _ = Builder::seeded(123).build(sim.build()).run();
}

struct ExternalReceiver {
    done: bool,
}
impl Module for ExternalReceiver {
    fn new() -> Self {
        Self { done: false }
    }
    fn handle_message(&mut self, _msg: Message) {
        self.done = true
    }
    fn at_sim_end(&mut self) {
        assert!(self.done)
    }
}

#[test]
#[serial_test::serial]
fn builder_with_extern() {
    let mut sim = AsyncBuilder::new();
    sim.set_default_cfg(NodeCfg { join: true });
    sim.node("client", |_| async {
        send(Message::new().build(), ("port", 0));
        Ok(())
    });
    sim.external::<ExternalReceiver>("server", None);
    sim.connect("client", "server");

    let _ = Builder::seeded(123).build(sim.build()).run();
}
