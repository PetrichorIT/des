#![cfg(feature = "net")]

use des::{
    net::{FailabilityPolicy, HandlerFn, ModuleBlock, ModuleFn},
    prelude::*,
};
use serial_test::serial;
use spin::Mutex;
use std::{
    hint::black_box,
    io,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
};

#[test]
#[serial]
fn builder_builds_hierachie() {
    let mut sim = Sim::new(());
    sim.node(
        "parent",
        ModuleFn::new(
            || {
                assert!(current().parent().is_err());
                assert!(current().child("child").is_ok());
            },
            |_, _| {},
        ),
    );
    sim.node(
        "parent.child",
        ModuleFn::new(
            || {
                assert!(current().parent().is_ok());
                assert!(current().child("grandchild").is_ok());
            },
            |_, _| {},
        ),
    );
    sim.node(
        "parent.child.grandchild",
        ModuleFn::new(
            || {
                assert!(current().parent().is_ok());
                assert!(current().child("some").is_err());
            },
            |_, _| {},
        ),
    );

    let _ = Builder::seeded(123).build(sim).run();
}

#[test]
#[serial]
#[should_panic = "cannot create node 'alice', node allready exists"]
fn builder_panic_node_duplicate() {
    let mut sim = Sim::new(());
    sim.node("alice", HandlerFn::new(|_| {}));
    sim.node("bob", HandlerFn::new(|_| {}));
    sim.node("alice", HandlerFn::new(|_| {}));
}

#[test]
#[serial]
#[should_panic = "cannot create node 'bob.bombardil', since parent node 'bob' is required, but does not exist"]
fn builder_panic_missing_parent() {
    let mut sim = Sim::new(());
    sim.node("alice", HandlerFn::new(|_| {}));
    sim.node("alice.alicent", HandlerFn::new(|_| {}));
    sim.node("bob.bombardil", HandlerFn::new(|_| {}));
}

#[test]
#[serial]
#[should_panic = "cannot create gate 'bob.port', because node 'bob' does not exist"]
fn builder_panic_gate_missing_node() {
    let mut sim = Sim::new(());
    sim.node("alice", HandlerFn::new(|_| {}));
    let _ = sim.gate("alice", "port");

    let _ = sim.gate("bob", "port");
}

#[test]
#[serial]
fn builder_gate_cluster() {
    struct Alice;
    impl Module for Alice {
        fn at_sim_start(&mut self, _: usize) {
            for i in 0..4 {
                assert!(current().gate("cluster", i).is_some());
            }
        }
    }

    let mut sim = Sim::new(());
    sim.node("alice", Alice);
    let _ = sim.gates("alice", "cluster", 4);

    let _ = Builder::seeded(123).build(sim).run();
}

#[test]
#[serial]
fn builder_module_block() {
    struct Def;
    struct Block;
    impl Module for Def {}
    impl ModuleBlock for Block {
        type Ret = ();
        fn build<A>(self, mut sim: ScopedSim<'_, A>) {
            sim.root(Def);
            let _ = sim.gate("", &format!("port-{}", sim.scope()));

            sim.node("sub", Def);
            let _ = sim.gates("sub", "cluster", 123);

            let _ = sim.inner();
        }
    }

    let mut sim = Sim::new(());
    sim.node("alice", Block);
    assert!(sim.get(&"alice.sub".into()).is_some());
}

#[test]
#[serial]
fn builder_handler_fn() {
    let counter = Arc::new(AtomicU16::new(0));
    let c2 = counter.clone();

    let mut sim = Sim::new(());
    sim.node(
        "alice",
        HandlerFn::new(move |msg| {
            c2.fetch_add(msg.header().id, Ordering::SeqCst);
        }),
    );
    let gate = sim.gate("alice", "port");
    let other = sim.gate("alice", "port");
    assert!(Arc::ptr_eq(&gate, &other));

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate.clone(), Message::default().id(1), 1.0.into());
    rt.add_message_onto(gate.clone(), Message::default().id(2), 2.0.into());
    rt.add_message_onto(gate.clone(), Message::default().id(3), 3.0.into());

    let _ = rt.run();
    assert_eq!(counter.load(Ordering::SeqCst), 6);
}

#[test]
#[serial]
fn builder_handler_fn_with_err() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        HandlerFn::failable(
            |_| {
                if black_box(false) {
                    return Err(io::Error::new(io::ErrorKind::Other, "other"));
                }

                Ok(())
            },
            FailabilityPolicy::Panic,
        ),
    );

    let _ = Builder::seeded(123).build(sim).run();
}

// #[test]
// #[serial]
// #[should_panic = "node 'alice' failed to process message, handler fn failed with: other"]
// fn builder_handler_fn_failure_panic() {
//     let mut sim = Sim::new(());
//     sim.node(
//         "alice",
//         HandlerFn::failable(
//             |_| {
//                 if black_box(true) {
//                     return Err(io::Error::new(io::ErrorKind::Other, "other"));
//                 }

//                 Ok(())
//             },
//             FailabilityPolicy::Panic,
//         ),
//     );
//     let gate = sim.gate("alice", "port");

//     let mut rt = Builder::seeded(123).build(sim);
//     rt.add_message_onto(gate, Message::default(), 1.0.into());

//     let _ = rt.run();
// }

#[test]
#[serial]
fn builder_handler_fn_failure_no_panic() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        HandlerFn::failable(
            |_| {
                if black_box(true) {
                    return Err(io::Error::new(io::ErrorKind::Other, "other"));
                }

                Ok(())
            },
            FailabilityPolicy::Continue,
        ),
    );
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate, Message::default(), 1.0.into());

    let _ = rt.run();
}

#[test]
#[serial]
fn builder_module_fn() {
    let records = Arc::new(Mutex::new(Vec::new()));
    let r2 = records.clone();

    let mut sim = Sim::new(());
    sim.node(
        "alice",
        ModuleFn::new(
            || 0,
            move |state, _| {
                *state += 1;
                if *state > 8 {
                    r2.lock().push(*state);
                }
            },
        ),
    );
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    for i in 0..10 {
        rt.add_message_onto(gate.clone(), Message::default().id(i), (i as f64).into());
    }

    let _ = rt.run();
    assert_eq!(*records.lock(), [9, 10]);
}

#[test]
#[serial]
fn builder_module_fn_restart_at_failure() {
    let starts = Arc::new(AtomicU16::new(0));
    let s2 = starts.clone();

    let mut sim = Sim::new(());
    sim.node(
        "alice",
        ModuleFn::failable(
            move || {
                s2.fetch_add(1, Ordering::SeqCst);
                0
            },
            |_, msg| {
                if msg.header().id == 1 {
                    Err(io::Error::new(io::ErrorKind::Other, "other"))
                } else {
                    Ok(())
                }
            },
            FailabilityPolicy::Restart,
        ),
    );
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate.clone(), Message::default().id(1), 1.0.into());
    rt.add_message_onto(gate.clone(), Message::default().id(1), 2.0.into());
    rt.add_message_onto(gate.clone(), Message::default().id(2), 3.0.into());

    let _ = rt.run();
    assert_eq!(starts.load(Ordering::SeqCst), 3);
}

#[test]
#[serial]
fn builder_module_fn_gen_in_module_scope() {
    let stage = Arc::new(AtomicU16::new(0));
    let s2 = stage.clone();

    let mut sim = Sim::new(());
    sim.node(
        "alice",
        ModuleFn::new(
            move || {
                s2.store(1, Ordering::SeqCst);
                assert_eq!(current().path().as_str(), "alice");

                123
            },
            |_, _| {},
        ),
    );

    assert_eq!(stage.load(Ordering::SeqCst), 0);
    let _ = Builder::seeded(123).build(sim).run();
    assert_eq!(stage.load(Ordering::SeqCst), 1);
}
