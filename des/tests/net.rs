use des::{
    net::{
        Error, Failure, globals,
        handlers::{AsyncHandler, ModuleFn},
        report,
    },
    prelude::*,
};
use serial_test::serial;

mod common;
pub use common::*;

#[derive(Default)]
struct Sender;

impl Module for Sender {
    fn at_sim_start(&mut self, _stage: usize) {
        for i in 0..10 {
            let _ = send_in(
                Message::default().with_id(i as u16),
                ("port", 0),
                Duration::from_secs(i),
            );
        }
    }
}

#[test]
#[serial]
fn connectivity() {
    let mut app = Sim::new(());

    app.node("rx", ExpectNMessage(10));
    app.node("tx", Sender::default());

    let rx = app.gate("rx", "port");
    let tx = app.gate("tx", "port");

    rx.connect_with(
        tx,
        Some(DatarateChannel::new(DatarateChannelMetrics {
            bitrate: 10000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::Queue(None),
        })),
    );

    let app = Builder::seeded(123).build(app.freeze());
    let _ = app.run().assert_no_err();
}

#[test]
#[serial]
fn select_node_from_globals() -> Result<(), Failure> {
    let mut sim = Sim::new(());

    sim.node("alice", NopModule);
    sim.node("alice.submodule", NopModule);
    sim.node("alice.submodule.child", NopModule);
    sim.node("bob", NopModule);

    sim.node(
        "tester",
        AsyncHandler::io(|_| async move {
            assert_eq!(globals().get(&"alice").unwrap().path(), "alice");
            assert_eq!(
                globals().get(&"alice.submodule").unwrap().path(),
                "alice.submodule"
            );
            assert_eq!(
                globals().get(&"alice.submodule.child").unwrap().path(),
                "alice.submodule.child"
            );
            assert_eq!(globals().get(&"bob").unwrap().path(), "bob");

            assert!(globals().get(&"steve").is_none());

            Ok(())
        }),
    );

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}

#[test]
#[serial]
fn can_access_foreign_module_context() -> Result<(), Failure> {
    let mut sim = Sim::new(());

    struct Alice;
    impl Module for Alice {
        fn at_sim_start(&mut self, _: usize) {
            current().prop::<String>("key").unwrap().set("value".into());
        }

        fn at_sim_end(&mut self) -> Result<(), Error> {
            assert_eq!(
                current().prop::<String>("key").unwrap().get(),
                Some("new_value".into())
            );
            Ok(())
        }
    }

    struct Bob;
    impl Module for Bob {
        fn num_sim_start_stages(&self) -> usize {
            2
        }

        fn at_sim_start(&mut self, s: usize) {
            if s == 0 {
                return;
            }
            let gate = current().gate("port").expect("local port must exist");
            let other = gate.path_end().expect("other module must exist").owner();

            // Gate parsing works just fine with IntoModuleGate
            let _ = other
                .gate("other-port")
                .expect("other port must exist and be resolved with the correct path");

            // simple data acces
            assert_eq!(other.gates().len(), 2);
            assert_eq!(other.path(), "alice");

            // prop access
            let mut prop = other.prop::<String>("key").unwrap();
            assert_eq!(prop.get(), Some("value".into()));
            prop.set("new_value".into());
            assert_eq!(prop.get(), Some("new_value".into()));

            // active
            assert!(!other.is_currently_active());
            assert!(current().is_currently_active());
        }
    }

    sim.node("alice", Alice);
    sim.node("bob", Bob);

    sim.gate("alice", "port").connect(sim.gate("bob", "port"));
    let _ = sim.gate("alice", "other-port");

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}

#[test]
#[serial]
fn custom_fail() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        ModuleFn::new(
            || schedule_at(Message::default(), 1.0.into()),
            |_, _| {
                report(std::io::Error::other("failed because i like to"));
            },
        ),
    );

    let err = Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .error
        .expect("expected an error");

    assert!(
        err[0]
            .to_string()
            .starts_with("alice: failed because i like to")
    );
}

#[test]
#[serial]
fn gate_disconnect() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(|_| async move {
            for _ in 0..5 {
                let _ = send(Message::default(), "a");
            }

            let gate = current().gate("a").unwrap();

            let peer = gate.next_gate().unwrap();
            gate.clone().disconnect(&peer);

            let _ = send(Message::default(), "a");

            let other = globals().get(&"charlie").unwrap().gate("c").unwrap();
            gate.connect(other);

            for _ in 0..7 {
                let _ = send(Message::default(), "a");
            }
        }),
    );
    sim.node("bob", ExpectNMessage(5));
    sim.node("charlie", ExpectNMessage(7));

    let a = sim.gate("alice", "a");
    let b = sim.gate("bob", "b");
    let _c = sim.gate("charlie", "c");

    a.connect(b);

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}

#[test]
#[serial]
#[should_panic = "cannot disconnect two unconnected gates"]
fn gate_disconnect_panic_at_unconnected() {
    let mut sim = Sim::new(());
    sim.node("alice", NopModule);
    let a = sim.gate("alice", "a");
    let b = sim.gate("alice", "b");

    a.disconnect(&b);
}
