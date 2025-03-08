#![cfg(feature = "net")]
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use des::{net::channel::ChannelProbe, prelude::*};
use rand::{rngs::StdRng, SeedableRng};
use serial_test::serial;

#[derive(Default)]
struct DropChanModule {
    send: usize,
    received: usize,
}

impl Module for DropChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::new().content([0u8; 512]).build(), "out");
        send(Message::new().content([1u8; 512]).build(), "out");

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

    let channel = Channel::new(ChannelMetrics {
        bitrate: 1000,
        latency: Duration::from_millis(100),
        jitter: Duration::ZERO,
        drop_behaviour: ChannelDropBehaviour::default(),
    });
    g_in.connect(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

#[derive(Debug, Default)]
struct BufferChanModule {
    send: usize,
    received: usize,
}

impl Module for BufferChanModule {
    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::new().content([0u8; 512]).build(), "out");
        send(Message::new().content([1u8; 512]).build(), "out");
        send(Message::new().content([1u8; 512]).build(), "out");

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
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = Sim::new(());
    rt.node("root", BufferChanModule::default());

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = Channel::new(ChannelMetrics {
        bitrate: 1000,
        latency: Duration::from_millis(100),
        jitter: Duration::ZERO,
        drop_behaviour: ChannelDropBehaviour::Queue(Some(600)),
    });
    g_in.connect(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

struct SendMessageModule;
impl Module for SendMessageModule {
    fn at_sim_start(&mut self, _stage: usize) {
        schedule_in(Message::new().kind(10).build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, msg: Message) {
        if msg.header().kind == 10 {
            send(Message::new().content("Hello world").build(), "out");
            let gate = current().gate("out", 0).unwrap();
            let ch = gate.channel().unwrap();
            assert!(ch.is_busy());
        }
    }
}

#[test]
#[serial]
fn channel_instant_busy() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = Sim::new(());
    rt.node("root", SendMessageModule);

    let g_in = rt.gate("root", "in");
    let g_out = rt.gate("root", "out");

    let channel = Channel::new(ChannelMetrics::new(
        1000,
        Duration::from_millis(100),
        Duration::ZERO,
        ChannelDropBehaviour::default(),
    ));

    g_in.connect(g_out, Some(channel));

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

#[derive(Default)]
struct ChannelProbing(Arc<AtomicUsize>);
impl Module for ChannelProbing {
    fn at_sim_start(&mut self, _stage: usize) {
        let chan = current().gate("port", 0).unwrap().channel().unwrap();

        chan.attach_probe(Probe(self.0.clone()));
        assert_eq!(chan.metrics().bitrate, 1234);

        let msg = Message::new().build();
        let busy_time = chan.calculate_busy(&msg);
        let tft = SimTime::now() + busy_time;
        dbg!(busy_time);
        assert_eq!(
            tft + Duration::from_millis(100),
            SimTime::from_duration(chan.calculate_duration(&msg, &mut StdRng::seed_from_u64(123)))
        );

        assert_eq!(chan.transmission_finish_time(), SimTime::MIN);
        assert_eq!(format!("{chan:?}"), format!("Channel {{ metrics: ChannelMetrics {{ bitrate: 1234, latency: 100ms, jitter: 0ns, drop_behaviour: Drop }}, state: Idle }}"));

        send(msg, "port");

        assert_eq!(chan.transmission_finish_time(), tft);
        assert_eq!(format!("{chan:?}"), format!("Channel {{ metrics: ChannelMetrics {{ bitrate: 1234, latency: 100ms, jitter: 0ns, drop_behaviour: Drop }}, state: Busy {{ until: {tft}, bytes: 0, packets: 0 }} }}"));
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.0.load(Ordering::SeqCst), 1);
        Ok(())
    }
}

struct Probe(Arc<AtomicUsize>);
impl ChannelProbe for Probe {
    fn on_message_transmit(&mut self, _: &ChannelMetrics, _: &Message) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
#[serial]
fn channel_probes() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = Sim::new(());
    rt.node("alice", ChannelProbing::default());
    rt.node("bob", ChannelProbing::default());

    let alice_port = rt.gate("alice", "port");
    let bob_port = rt.gate("bob", "port");

    let chan = Channel::new(ChannelMetrics {
        bitrate: 1234,
        latency: Duration::from_millis(100),
        jitter: Duration::ZERO,
        drop_behaviour: ChannelDropBehaviour::default(),
    });

    alice_port.connect(bob_port, Some(chan));

    let rt = Builder::seeded(123).build(rt);
    let _ = rt.run();
}

struct LatencyOnly(usize);

impl Module for LatencyOnly {
    fn at_sim_start(&mut self, _stage: usize) {
        for _ in 0..10 {
            send(Message::new().build(), "out");
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
    gout.connect(
        gin,
        Some(Channel::new(ChannelMetrics::new(
            0,
            Duration::from_secs(1),
            Duration::ZERO,
            ChannelDropBehaviour::Drop,
        ))),
    );

    let _ = Builder::seeded(123).build(sim).run();
}
