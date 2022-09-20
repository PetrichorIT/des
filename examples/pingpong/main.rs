use des::{net::gate::GateServiceType, prelude::*};

mod alice;
mod bob;

#[derive(Debug)]
struct Application();

#[allow(clippy::cmp_owned)]
fn main() {
    use des::net::{BuildContext, __Buildable0};

    let mut app = NetworkRuntime::new(Application());
    let mut cx = BuildContext::new(&mut app);

    let bob = bob::Bob::build_named(ObjectPath::root_module("bob".to_string()), &mut cx);
    let alice = alice::Alice::build_named_with_parent("alice", bob.clone(), &mut cx);

    let channel = Channel::new(
        ObjectPath::new("pingpong#chan".to_string()).unwrap(),
        ChannelMetrics::new(5_000_000, Duration::from_secs_f64(0.1), Duration::new(0, 0)),
    );

    let g1 = alice.create_gate("netIn", GateServiceType::Input);
    let g4 = bob.create_gate_into(
        "netOut",
        GateServiceType::Output,
        Some(channel.clone()),
        Some(g1),
    );

    let r1 = bob.create_gate("netIn", GateServiceType::Input);
    let _r4 = alice.create_gate_into("netOut", GateServiceType::Output, Some(channel), Some(r1));

    cx.create_module(alice);
    cx.create_module(bob);

    let mut rt = Runtime::new_with(app, RuntimeOptions::seeded(0x56123).max_time(420.0.into()));

    let msg = Message::new().content("Ping".to_string()).build();

    rt.add_message_onto(g4, msg, 1.0.into());

    let (_, time, p, _) = rt.run().unwrap_premature_abort();

    assert_eq!(time.as_secs(), 419);
    assert_eq!(p.event_count, 16744);
}
