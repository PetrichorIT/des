use des::{net::GateServiceType, prelude::*};

mod alice;
mod bob;

#[derive(Debug)]
struct Application();

#[allow(clippy::cmp_owned)]
fn main() {
    let mut alice = PtrMut::new(alice::Alice(ModuleCore::new()));
    let mut bob = PtrMut::new(bob::Bob(ModuleCore::new()));

    bob.add_child(&mut alice);

    let mut app = NetworkRuntime::new(Application());

    let channel = Channel::new(
        ObjectPath::new("pingpong#chan".to_string()).unwrap(),
        ChannelMetrics::new(5_000_000, Duration::from_secs_f64(0.1), Duration::new(0, 0)),
    );

    let g1 = alice.create_gate("netIn", GateServiceType::Input, &mut app);
    let g4 = bob.create_gate_into(
        "netOut",
        GateServiceType::Output,
        Some(channel.clone()),
        Some(g1),
        &mut app,
    );

    let r1 = bob.create_gate("netIn", GateServiceType::Input, &mut app);
    let _r4 = alice.create_gate_into(
        "netOut",
        GateServiceType::Output,
        Some(channel),
        Some(r1),
        &mut app,
    );

    app.create_module(alice);
    app.create_module(bob);

    let mut rt = Runtime::new_with(app, RuntimeOptions::seeded(0x56123).max_time(420.0.into()));

    let msg = Message::new().content("Ping".to_string()).build();

    rt.add_message_onto(g4, msg, 1.0.into());

    let (_, time, p, _) = rt.run().unwrap_premature_abort();

    assert_eq_time!(time, 419.926816);
    assert_eq!(p.event_count, 16760);
}
