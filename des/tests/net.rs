use des::prelude::*;

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
                Message::new().id(i as u16).build(),
                ("port", 0),
                Duration::from_secs(i),
            );
        }
    }
}

#[test]
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
