use std::net::{IpAddr, Ipv4Addr};

use des::{assert_eq_time, prelude::*};
use network_node::NetworkNode;
use network_stack::NetworkStack;
use routing_deamon::RandomRoutingDeamon;

mod network_node;
mod network_stack;
mod routing_deamon;

struct A();

fn main() {
    // === Internal mapping ===
    //
    // node::netOut --> ... (later)
    // node::netIn  <-- ... (later)
    // node::toStack --> stack::netIn
    // node::fromStack  <-- stack::netOut

    let mut app = NetworkRuntime::new(A());

    //
    // ALICE
    //
    let mut node_alice = PtrMut::new(NetworkNode::named("Alice", app.globals_weak()));
    let mut stack_alice = NetworkStack::new(
        "Alice.NetworkStack",
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 120)),
        RandomRoutingDeamon::new(app.globals_weak()),
    );
    node_alice.add_child(&mut stack_alice);

    let internal_out = node_alice.create_gate("fromStack", GateServiceType::Input, &mut app);

    stack_alice.create_gate_into(
        "netOut",
        GateServiceType::Output,
        None,
        Some(internal_out),
        &mut app,
    );

    let internal_in = stack_alice.create_gate("netIn", GateServiceType::Input, &mut app);

    node_alice.create_gate_into(
        "toStack",
        GateServiceType::Output,
        None,
        Some(internal_in),
        &mut app,
    );

    //
    // BOB
    //
    let mut node_bob = PtrMut::new(NetworkNode::named("Bob", app.globals_weak()));
    let mut stack_bob = NetworkStack::new(
        "Bob.NetworkStack",
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 100)),
        RandomRoutingDeamon::new(app.globals_weak()),
    );

    node_bob.add_child(&mut stack_bob);

    let internal_out = node_bob.create_gate("fromStack", GateServiceType::Input, &mut app);

    stack_bob.create_gate_into(
        "netOut",
        GateServiceType::Output,
        None,
        Some(internal_out),
        &mut app,
    );

    let internal_in = stack_bob.create_gate("netIn", GateServiceType::Input, &mut app);

    node_bob.create_gate_into(
        "toStack",
        GateServiceType::Output,
        None,
        Some(internal_in),
        &mut app,
    );

    //
    // EVE
    //

    let mut node_eve = PtrMut::new(NetworkNode::named("Eve", app.globals_weak()));
    let mut stack_eve = NetworkStack::new(
        "Eve.NetworkStack",
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 110)),
        RandomRoutingDeamon::new(app.globals_weak()),
    );

    node_eve.add_child(&mut stack_eve);

    let internal_out =
        node_eve.create_gate_cluster("fromStack", 2, GateServiceType::Input, &mut app);

    stack_eve.create_gate_cluster_into(
        "netOut",
        2,
        GateServiceType::Output,
        None,
        internal_out.into_iter().map(Some).collect(),
        &mut app,
    );

    let internal_in = stack_eve.create_gate_cluster("netIn", 2, GateServiceType::Input, &mut app);

    node_eve.create_gate_cluster_into(
        "toStack",
        2,
        GateServiceType::Output,
        None,
        internal_in.into_iter().map(Some).collect(),
        &mut app,
    );

    //
    // Application config
    //

    let channel = Some(Channel::new(
        ObjectPath::new("inet#chan".to_string()).unwrap(),
        ChannelMetrics::new(
            5_000_000,
            Duration::from_secs_f64(0.1),
            Duration::from_secs_f64(0.0),
        ),
    ));

    let alice_in = node_alice.create_gate("channelIncoming", GateServiceType::Input, &mut app);

    let bob_in = node_bob.create_gate("channelIncoming", GateServiceType::Input, &mut app);

    node_eve.create_gate_cluster_into(
        "channelOutgoing",
        2,
        GateServiceType::Output,
        channel.clone(),
        vec![Some(alice_in.clone()), Some(bob_in)],
        &mut app,
    );

    let eve_in =
        node_eve.create_gate_cluster("channelIncoming", 2, GateServiceType::Input, &mut app);

    node_alice.create_gate_into(
        "channelOutgoing",
        GateServiceType::Output,
        channel.clone(),
        Some(eve_in[0].clone()),
        &mut app,
    );
    node_bob.create_gate_into(
        "channelOutgoing",
        GateServiceType::Output,
        channel,
        Some(eve_in[1].clone()),
        &mut app,
    );

    app.create_module(node_alice);
    app.create_module(stack_alice);

    app.create_module(node_bob);
    app.create_module(stack_bob);

    app.create_module(node_eve);
    app.create_module(stack_eve);

    let mut rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    let msg = Message::new()
        // .src(0x_00_00_00_ff, 0x00_fe)
        // .dest(0x00_00_00_ee, 0x00_fe)
        .content("PING".to_string())
        .kind(2)
        .build();

    // let msg = Message::legacy_new(
    //     0,
    //     2,
    //     None,
    //     ModuleId::NULL,
    //     ModuleId::NULL,
    //     SimTime::ZERO,
    //     pkt,
    // );

    rt.add_message_onto(alice_in, msg, 0.0.into());

    let (app, time, p) = rt.run().unwrap();

    let _ = app
        .globals()
        .topology
        .borrow()
        .write_to_svg("examples/inet/graph");

    assert_eq_time!(time, 0.200127998);
    assert_eq!(p.event_count, 21);
}
