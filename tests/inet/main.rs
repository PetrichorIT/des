use des::{
    Channel, ChannelMetrics, Message, ModuleId, Mrc, NetworkRuntime, Packet, Runtime, SimTime,
};
use des::{GateId, StaticModuleCore};
use network_node::NetworkNode;
use network_stack::NetworkStack;
use rand::{prelude::StdRng, SeedableRng};
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
    let mut node_alice = Mrc::new(NetworkNode::named("Alice", app.parameters()));
    let mut stack_alice =
        NetworkStack::new(0x00_00_00_ff, RandomRoutingDeamon::new(app.parameters()));
    node_alice.add_child(&mut *stack_alice);

    let internal_out = node_alice.create_gate("fromStack", &mut app);

    stack_alice.create_gate_into("netOut", None, Some(internal_out), &mut app);

    let internal_in = stack_alice.create_gate("netIn", &mut app);

    node_alice.create_gate_into("toStack", None, Some(internal_in), &mut app);

    //
    // BOB
    //
    let mut node_bob = Mrc::new(NetworkNode::named("Bob", app.parameters()));
    let mut stack_bob =
        NetworkStack::new(0x00_00_00_ee, RandomRoutingDeamon::new(app.parameters()));

    node_bob.add_child(&mut *stack_bob);

    let internal_out = node_bob.create_gate("fromStack", &mut app);

    stack_bob.create_gate_into("netOut", None, Some(internal_out), &mut app);

    let internal_in = stack_bob.create_gate("netIn", &mut app);

    node_bob.create_gate_into("toStack", None, Some(internal_in), &mut app);

    //
    // EVE
    //

    let mut node_eve = Mrc::new(NetworkNode::named("Eve", app.parameters()));
    let mut stack_eve =
        NetworkStack::new(0x00_00_00_dd, RandomRoutingDeamon::new(app.parameters()));

    node_eve.add_child(&mut *stack_eve);

    let internal_out = node_eve.create_gate_cluster("fromStack", 2, &mut app);

    stack_eve.create_gate_cluster_into(
        "netOut",
        2,
        None,
        internal_out.into_iter().map(Some).collect(),
        &mut app,
    );

    let internal_in = stack_eve.create_gate_cluster("netIn", 2, &mut app);

    node_eve.create_gate_cluster_into(
        "toStack",
        2,
        None,
        internal_in.into_iter().map(Some).collect(),
        &mut app,
    );

    //
    // Application config
    //

    let channel = Some(Channel::new(ChannelMetrics {
        bitrate: 5_000_000,
        latency: 0.1.into(),
        jitter: 0.0.into(),
    }));

    let alice_in = node_alice.create_gate("channelIncoming", &mut app);

    let bob_in = node_bob.create_gate("channelIncoming", &mut app);

    node_eve.create_gate_cluster_into(
        "channelOutgoing",
        2,
        channel.clone(),
        vec![Some(alice_in.clone()), Some(bob_in)],
        &mut app,
    );

    let eve_in = node_eve.create_gate_cluster("channelIncoming", 2, &mut app);

    node_alice.create_gate_into(
        "channelOutgoing",
        channel.clone(),
        Some(eve_in[0].clone()),
        &mut app,
    );
    node_bob.create_gate_into(
        "channelOutgoing",
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

    let mut rt = Runtime::new_with(
        app,
        des::RuntimeOptions {
            rng: StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    let pkt = Packet::new(
        (0x00_00_00_ff, 0x00_fe),
        (0x00_00_00_ee, 0x00_fe),
        String::from("PING"),
    );
    let msg = Message::new(
        0,
        2,
        GateId::NULL,
        ModuleId::NULL,
        ModuleId::NULL,
        SimTime::ZERO,
        pkt,
    );

    rt.add_message_onto(alice_in, msg, 0.0.into());

    rt.run();
}
