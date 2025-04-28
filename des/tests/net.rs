use des::{
    net::{
        blocks::{AsyncFn, HandlerFn},
        globals,
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
            send_in(
                Message::default().id(i as u16),
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

    rx.connect(
        tx,
        Some(Channel::new(ChannelMetrics {
            bitrate: 10000,
            latency: Duration::from_millis(100),
            jitter: Duration::ZERO,
            drop_behaviour: ChannelDropBehaviour::Queue(None),
        })),
    );

    let app = Builder::seeded(123).build(app);
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
        AsyncFn::io(|_| async move {
            assert_eq!(globals().node("alice").unwrap().path(), "alice".into());
            assert_eq!(
                globals().node("alice.submodule").unwrap().path(),
                "alice.submodule".into()
            );
            assert_eq!(
                globals().node("alice.submodule.child").unwrap().path(),
                "alice.submodule.child".into()
            );
            assert_eq!(globals().node("bob").unwrap().path(), "bob".into());

            assert!(globals().node("steve").is_err());

            Ok(())
        }),
    );

    Builder::seeded(123).build(sim).run().map(|_| ())
}
