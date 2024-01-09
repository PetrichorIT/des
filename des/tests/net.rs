use des::prelude::*;

#[macro_use]
mod common;

struct Receiver {
    counter: usize,
}
impl_build_named!(Receiver);

impl Module for Receiver {
    fn new() -> Self {
        Self { counter: 0 }
    }

    fn handle_message(&mut self, _msg: Message) {
        self.counter += 1;
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.counter, 10);
    }
}

struct Sender;
impl_build_named!(Sender);

impl Module for Sender {
    fn new() -> Self {
        Self
    }

    fn at_sim_start(&mut self, _stage: usize) {
        for i in 0..10 {
            println!("sending {i}: {:?}", current().gates());
            send_in(
                Message::new().id(i as u16).build(),
                "port",
                Duration::from_secs(i),
            );
        }
    }
}

#[test]
fn connectivity() {
    let mut app = NetworkApplication::new(());
    let rx = Receiver::build_named("rx".into(), &mut app);
    let tx = Sender::build_named("tx".into(), &mut app);

    let rxg = rx.create_gate("port");
    let txg = tx.create_gate("port");
    rxg.connect(
        txg,
        Some(Channel::new(
            "chan".into(),
            ChannelMetrics {
                bitrate: 10000,
                latency: Duration::from_millis(100),
                jitter: Duration::ZERO,
                drop_behaviour: ChannelDropBehaviour::Queue(None),
            },
        )),
    );

    app.register_module(rx);
    app.register_module(tx);

    let app = Builder::seeded(123).build(app);
    let _ = app.run().unwrap();
}
