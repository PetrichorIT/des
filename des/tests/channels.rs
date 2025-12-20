#![cfg(feature = "net")]

use des::{
    net::{
        channel::DelayChannel,
        handlers::{AsyncHandler, HandlerFn},
        internals::{MessageExitingConnection, NetEvents},
    },
    prelude::*,
    time::sleep_until,
};
use serial_test::serial;
use tokio::spawn;

#[derive(Default)]
struct DropChanModule {
    send: usize,
    received: usize,
}

impl Module for DropChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        let _ = send(Message::default().with_content([0u8; 512]), "out");
        let _ = send(Message::default().with_content([1u8; 512]), "out");

        self.send += 2;
    }

    fn handle_message(&mut self, _msg: Message) {
        self.received += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_ne!(self.send, self.received);
        Ok(())
    }
}

#[test]
#[serial]
fn channel_dropping_message() {
    let mut rt = Sim::new(());
    rt.node("root", DropChanModule::default());

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = DatarateChannel::new(DatarateChannelMetrics {
        bitrate: 1000,
        latency: Duration::from_millis(100),
        jitter: Duration::ZERO,
        drop_behaviour: ChannelDropBehaviour::default(),
    });
    g_in.connect_with(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt.freeze());
    let _ = rt.run();
}

#[derive(Debug, Default)]
struct BufferChanModule {
    send: usize,
    received: usize,
}

impl Module for BufferChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        let _ = send(Message::default().with_content([0u8; 512]), "out");
        let _ = send(Message::default().with_content([1u8; 512]), "out");
        let _ = send(Message::default().with_content([1u8; 512]), "out");

        self.send += 3;
    }

    fn handle_message(&mut self, _msg: Message) {
        self.received += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.send, 3);
        assert_eq!(self.received, 2);
        Ok(())
    }
}

#[test]
#[serial]
fn channel_buffering_message() {
    let mut rt = Sim::new(());
    rt.node("root", BufferChanModule::default());

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = DatarateChannel::new(DatarateChannelMetrics {
        bitrate: 1000,
        latency: Duration::from_millis(100),
        jitter: Duration::ZERO,
        drop_behaviour: ChannelDropBehaviour::Queue(Some(600)),
    });
    g_in.connect_with(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt.freeze());
    let _ = rt.run();
}

struct SendMessageModule;
impl Module for SendMessageModule {
    fn at_sim_start(&mut self, _stage: usize) {
        schedule_in(Message::default().with_kind(10), Duration::from_secs(1));
    }

    fn handle_message(&mut self, msg: Message) {
        if msg.header.kind == 10 {
            let _ = send(Message::default().with_content("Hello world"), "out");
            let gate = current().gate("out").unwrap();
            let ch = gate.channel().unwrap();
            assert!(ch.is_busy());
        }
    }
}

#[test]
#[serial]
fn channel_instant_busy() {
    let mut rt = Sim::new(());
    rt.node("root", SendMessageModule);

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = DatarateChannel::new(DatarateChannelMetrics::new(
        1000,
        Duration::from_millis(100),
        Duration::ZERO,
        ChannelDropBehaviour::default(),
    ));

    g_in.connect_with(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt.freeze());
    let _ = rt.run();
}

struct LatencyOnly(usize);

impl Module for LatencyOnly {
    fn at_sim_start(&mut self, _stage: usize) {
        for _ in 0..10 {
            let _ = send(Message::default(), "out");
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        self.0 += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.0, 10);
        Ok(())
    }
}

#[test]
#[serial]
fn latency_only_channel() {
    let mut sim = Sim::new(());
    sim.node("alice", LatencyOnly(0));
    let gout = sim.gate("alice", "out");
    let gin = sim.gate("alice", "in");
    gout.connect_with(
        gin,
        Some(DatarateChannel::new(DatarateChannelMetrics::new(
            1000,
            Duration::from_secs(1),
            Duration::ZERO,
            ChannelDropBehaviour::Drop,
        ))),
    );

    let _ = Builder::seeded(123).build(sim.freeze()).run();
}

#[test]
#[serial]
fn simplex_shared_domain() {
    let mut sim = Sim::new(());

    sim.node(
        "receiver",
        AsyncHandler::new(|mut rx| async move {
            for _ in 0..25 {
                let msg = rx.recv().await.unwrap();
                let last = msg.last_gate.clone().unwrap();
                let _ = send(msg, last);
                tracing::info!("received message");
            }
        })
        .require_join(),
    );

    let switch = sim.gates("receiver", "mobile", 5);
    let to_receiver = ChannelRef::from(DatarateChannel::new(DatarateChannelMetrics::new(
        64 * 8,
        Duration::ZERO,
        Duration::ZERO,
        ChannelDropBehaviour::Queue(None),
    )));
    let to_sender = ChannelRef::from(DatarateChannel::new(DatarateChannelMetrics::new(
        64 * 8,
        Duration::ZERO,
        Duration::ZERO,
        ChannelDropBehaviour::Queue(None),
    )));

    for i in 0..5 {
        let key = format!("sender-{i}");
        sim.node(
            &key,
            AsyncHandler::new(|_| async move {
                for _ in 0..5 {
                    let _ = send(Message::default(), "mobile");
                }
            }),
        );
        let gate = sim.gate(key, "mobile");

        gate.clone().connect_with(
            switch[i].clone(),
            Some((to_receiver.clone(), to_sender.clone())),
        );
    }

    let rt = Builder::seeded(123).build(sim.freeze()).run().unwrap();

    //
    assert_eq!(rt.2.event_count, 130);
    assert_eq!(rt.1, 26.0)
}

#[test]
#[serial]
fn duplex_shared_domain() {
    let mut sim = Sim::new(());

    sim.node(
        "receiver",
        AsyncHandler::new(|mut rx| async move {
            for _ in 0..25 {
                let msg = rx.recv().await.unwrap();
                let last = msg.last_gate.clone().unwrap();
                let _ = send(msg, last);
                tracing::info!("received message");
            }
        })
        .require_join(),
    );

    let switch = sim.gates("receiver", "mobile", 5);
    let chan = ChannelRef::from(DatarateChannel::new(DatarateChannelMetrics::new(
        64 * 8,
        Duration::ZERO,
        Duration::ZERO,
        ChannelDropBehaviour::Queue(None),
    )));

    for i in 0..5 {
        let key = format!("sender-{i}");
        sim.node(
            &key,
            AsyncHandler::new(|_| async move {
                for _ in 0..5 {
                    let _ = send(Message::default(), "mobile");
                }
            }),
        );
        let gate = sim.gate(key, "mobile");

        gate.clone()
            .connect_with(switch[i].clone(), Some(chan.clone()));
    }

    let rt = Builder::seeded(123).build(sim.freeze()).run().unwrap();

    // 50 messages over datarate channel with infinite buffer
    // - 50 handle message events
    // - 50 Exiting + 50 handle message events
    // - 48 Channel Notif (only from once two events in queue, thus not for first and not for last message)
    // - 6 start signals

    assert_eq!(rt.2.event_count, 50 + 50 + 48 + 6);
    assert_eq!(rt.1, 50.0)
}

#[test]
#[serial]
fn channel_as_any() {
    let mut sim = Sim::new(());
    sim.node("alice", ());
    sim.node("bob", ());

    let g1 = sim.gate("alice", "port");
    let g2 = sim.gate("bob", "port");

    g1.clone()
        .connect_with(g2, Some(DelayChannel::new(Duration::from_millis(100))));
    let ch = g1.channel().unwrap();

    assert_eq!(
        ch.downcast_ref::<DelayChannel, _>(|v| v.delay).unwrap(),
        Duration::from_millis(100)
    );
}

struct Empty;
impl Module for Empty {}

#[test]
#[serial]
fn datarate_channel_can_send_at_tft_independent_of_event_order() {
    let mut sim = Sim::new(());
    let mut metrics = DatarateChannelMetrics {
        bitrate: 64,
        latency: Duration::from_millis(100),
        jitter: Duration::ZERO,
        drop_behaviour: ChannelDropBehaviour::Drop,
    };

    sim.node(
        "alice",
        AsyncHandler::new(move |mut rx| async move {
            let offset = Duration::from_secs(1);
            let tft = SimTime::from_duration(metrics.calculate_busy(&Message::default())) + offset;

            // to get the "wrong" event order we must shedule the spurious wakeup before the channel unbusy
            let handle = spawn(async move {
                sleep_until(tft).await;

                // channel(s) has not yet received the unbusy event
                current()
                    .gate("port-no-buffer")
                    .unwrap()
                    .channel()
                    .unwrap()
                    .downcast_ref::<DatarateChannel, _>(|drc| {
                        assert_eq!(drc.transmission_finish_time(), Some(tft));
                    })
                    .unwrap();

                // however sending should still work
                send(Message::default(), "port-no-buffer").expect("success");
                send(Message::default(), "port-buffer").expect("success");
            });

            // funny buisness to ensure sleep_until in the task actually create a wakeup event.
            schedule_in(Message::default(), offset);
            let _ = rx.recv().await.unwrap();

            send(Message::default(), "port-no-buffer").expect("success");
            send(Message::default(), "port-buffer").expect("success");

            sleep_until(tft / 2.0).await;
            // both channels are still busy
            send(Message::default(), "port-no-buffer").expect_err("channel should block");
            send(Message::default(), "port-buffer").expect("one message can be buffered");
            send(Message::default(), "port-buffer").expect_err("but not another one");

            handle.await.unwrap();
        })
        .require_join(),
    );

    sim.node("bob", Empty);

    let g1 = sim.gate("alice", "port-no-buffer");
    let g2 = sim.gate("alice", "port-buffer");

    let t1 = sim.gate("bob", "port-no-buffer");
    let t2 = sim.gate("bob", "port-buffer");

    let no_buffer = DatarateChannel::new(metrics.clone());

    metrics.drop_behaviour = ChannelDropBehaviour::Queue(Some(64)); // 64 bytes

    let buffer = DatarateChannel::new(metrics);

    g1.connect_with(t1, Some(no_buffer));
    g2.connect_with(t2, Some(buffer));

    let _ = Builder::seeded(123).build(sim.freeze()).run().unwrap();
}

#[derive(Debug, Clone, Default)]
struct CustomFwdChannel {
    peers: Vec<GateRef>,
}

impl Channel for CustomFwdChannel {
    fn register(&mut self, endpoint: GateRef) {
        self.peers.push(endpoint);
        self.peers.dedup();
    }

    fn unregister(&mut self, endpoint: GateRef) {
        self.peers.retain(|v| *v != endpoint);
    }

    fn send(
        &mut self,
        _: GateRef,
        msg: Message,
        via: des::net::gate::Connection,
        ctx: des::net::channel::SendContext<'_>,
    ) -> Result<(), SendError> {
        ctx.sink.add(
            NetEvents::MessageExitingConnection(MessageExitingConnection { con: via, msg }),
            SimTime::now() + Duration::from_secs(1),
        );
        Ok(())
    }

    fn transmission_finish_time(&self) -> Option<SimTime> {
        None
    }

    fn unbusy_notify(
        &mut self,
        _: Box<dyn std::any::Any + Send>,
        _: des::net::channel::SendContext<'_>,
    ) {
    }
}

#[test]
#[serial]
fn register_unregister_custom_channel() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::new(|_| async move {
            for _ in 0..5 {
                let _ = send(Message::default(), "a");
                let _ = send(Message::default(), "a2");
            }

            let a = current().gate("a").unwrap();

            assert_eq!(
                a.channel()
                    .unwrap()
                    .downcast_ref(|c: &CustomFwdChannel| c.peers.len())
                    .unwrap_or(0),
                4
            );

            let peer = a.next_gate().unwrap();
            a.clone().disconnect(&peer);

            let a2 = current().gate("a2").unwrap();

            assert_eq!(
                a2.channel()
                    .unwrap()
                    .downcast_ref(|c: &CustomFwdChannel| c.peers.len())
                    .unwrap_or(0),
                2
            );
        }),
    );
    sim.node("bob", HandlerFn::new(|_| ()));
    sim.node("charlie", HandlerFn::new(|_| ()));

    let a = sim.gate("alice", "a");
    let a2 = sim.gate("alice", "a2");
    let b = sim.gate("bob", "b");
    let c = sim.gate("charlie", "c");

    let chan = ChannelRef::from(CustomFwdChannel::default());

    a.connect_with(b, Some(chan.clone()));
    a2.connect_with(c, Some(chan));

    Builder::seeded(123).build(sim.freeze()).run().map(|_| ())
}
