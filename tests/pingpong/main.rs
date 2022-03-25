use des::prelude::*;

mod alice;
mod bob;

#[derive(Debug)]
struct Application();

#[allow(clippy::cmp_owned)]
fn main() {
    let mut alice = Mrc::new(alice::Alice(ModuleCore::new()));
    let mut bob = Mrc::new(bob::Bob(ModuleCore::new()));

    bob.add_child(&mut alice);

    let mut app = NetworkRuntime::new(Application());

    let channel = Channel::new(ChannelMetrics {
        bitrate: 5_000_000,
        latency: 0.1.into(),
        jitter: 0.0.into(),
    });

    let g1 = alice.create_gate("netIn", &mut app);
    let g4 = bob.create_gate_into("netOut", Some(channel.clone()), Some(g1), &mut app);

    let r1 = bob.create_gate("netIn", &mut app);
    let _r4 = alice.create_gate_into("netOut", Some(channel), Some(r1), &mut app);

    app.create_module(alice);
    app.create_module(bob);

    let mut rt = Runtime::new_with(app, RuntimeOptions::seeded(0x56123).max_time(420.0.into()));

    let msg = Message::new(
        0,
        1,
        None,
        ModuleId::NULL,
        ModuleId::NULL,
        SimTime::now(),
        String::from("Ping"),
    );

    rt.add_message_onto(g4, msg, 1.0.into());

    let (_, time, event_count, _) = rt.run().unwrap_premature_abort();

    assert!(time < SimTime::from(420.0));
    assert_eq!(event_count, 16760);
}
