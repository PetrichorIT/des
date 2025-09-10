use des::{
    net::{
        globals,
        handlers::{AsyncHandler, HandlerFn},
    },
    prelude::*,
};
use serial_test::serial;

#[derive(Default)]
struct Receiver {
    counter: usize,
}

impl Module for Receiver {
    fn handle_message(&mut self, _msg: Message) {
        self.counter += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.counter, 10);
        Ok(())
    }
}

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

    app.node("rx", Receiver::default());
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
    let _ = app.run().unwrap();
}

#[test]
#[serial]
fn select_node_from_globals() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());

    sim.node("alice", HandlerFn::new(|_| {}));
    sim.node("alice.submodule", HandlerFn::new(|_| {}));
    sim.node("alice.submodule.child", HandlerFn::new(|_| {}));
    sim.node("bob", HandlerFn::new(|_| {}));

    sim.node(
        "tester",
        AsyncHandler::io(|_| async move {
            assert_eq!(
                globals().get(&"alice".into()).unwrap().path(),
                "alice".into()
            );
            assert_eq!(
                globals().get(&"alice.submodule".into()).unwrap().path(),
                "alice.submodule".into()
            );
            assert_eq!(
                globals()
                    .get(&"alice.submodule.child".into())
                    .unwrap()
                    .path(),
                "alice.submodule.child".into()
            );
            assert_eq!(globals().get(&"bob".into()).unwrap().path(), "bob".into());

            assert!(globals().get(&"steve".into()).is_none());

            Ok(())
        }),
    );

    Builder::seeded(123).build(sim.freeze()).run().map(|_| ())
}

#[test]
#[serial]
fn can_access_foreign_module_context() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());

    struct Alice;
    impl Module for Alice {
        fn at_sim_start(&mut self, _: usize) {
            current().prop::<String>("key").unwrap().set("value".into());
        }

        fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
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
            assert_eq!(other.path(), "alice".into());

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

    Builder::seeded(123).build(sim.freeze()).run().map(|_| ())
}
