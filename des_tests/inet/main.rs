use std::mem::ManuallyDrop;

use des_core::{
    Channel, ChannelMetrics, Message, MessageAtGateEvent, Module, ModuleExt, NetworkRuntime,
    Packet, Runtime, SimTime, GATE_NULL, MODULE_NULL,
};
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

    //
    // ALICE
    //
    let mut node_alice = Box::new(NetworkNode::named("Alice"));
    let mut stack_alice = NetworkStack::new(0x00_00_00_ff, RandomRoutingDeamon::new());
    stack_alice.set_parent(&mut node_alice);

    let internal_out = node_alice.create_gate(
        String::from("fromStack"),
        des_core::GateType::Input,
        &Channel::INSTANTANEOUS,
    );

    stack_alice.create_gate_into(
        String::from("netOut"),
        des_core::GateType::Output,
        &Channel::INSTANTANEOUS,
        internal_out,
    );

    let internal_in = stack_alice.create_gate(
        String::from("netIn"),
        des_core::GateType::Input,
        &Channel::INSTANTANEOUS,
    );

    node_alice.create_gate_into(
        String::from("toStack"),
        des_core::GateType::Output,
        &Channel::INSTANTANEOUS,
        internal_in,
    );

    //
    // BOB
    //
    let mut node_bob = Box::new(NetworkNode::named("Bob"));
    let mut stack_bob = NetworkStack::new(0x00_00_00_ee, RandomRoutingDeamon::new());

    stack_bob.set_parent(&mut node_bob);

    let internal_out = node_bob.create_gate(
        String::from("fromStack"),
        des_core::GateType::Input,
        &Channel::INSTANTANEOUS,
    );

    stack_bob.create_gate_into(
        String::from("netOut"),
        des_core::GateType::Output,
        &Channel::INSTANTANEOUS,
        internal_out,
    );

    let internal_in = stack_bob.create_gate(
        String::from("netIn"),
        des_core::GateType::Input,
        &Channel::INSTANTANEOUS,
    );

    node_bob.create_gate_into(
        String::from("toStack"),
        des_core::GateType::Output,
        &Channel::INSTANTANEOUS,
        internal_in,
    );

    //
    // EVE
    //

    let mut node_eve = Box::new(NetworkNode::named("Eve"));
    let mut stack_eve = NetworkStack::new(0x00_00_00_dd, RandomRoutingDeamon::new());

    stack_eve.set_parent(&mut node_eve);

    let internal_out = node_eve.create_gate_cluster(
        String::from("fromStack"),
        2,
        des_core::GateType::Input,
        &Channel::INSTANTANEOUS,
    );

    stack_eve.create_gate_cluster_into(
        String::from("netOut"),
        2,
        des_core::GateType::Output,
        &Channel::INSTANTANEOUS,
        internal_out,
    );

    let internal_in = stack_eve.create_gate_cluster(
        String::from("netIn"),
        2,
        des_core::GateType::Input,
        &Channel::INSTANTANEOUS,
    );

    node_eve.create_gate_cluster_into(
        String::from("toStack"),
        2,
        des_core::GateType::Output,
        &Channel::INSTANTANEOUS,
        internal_in,
    );

    //
    // Application config
    //

    let mut app = NetworkRuntime::new(A());

    let channel = app.create_channel(ChannelMetrics {
        bitrate: 5_000_000,
        latency: 0.1.into(),
        jitter: 0.0.into(),
    });

    let alice_in = node_alice.create_gate(
        String::from("channelIncoming"),
        des_core::GateType::Input,
        channel,
    );

    let bob_in = node_bob.create_gate(
        String::from("channelIncoming"),
        des_core::GateType::Input,
        channel,
    );

    node_eve.create_gate_cluster_into(
        String::from("channelOutgoing"),
        2,
        des_core::GateType::Output,
        channel,
        vec![alice_in, bob_in],
    );

    let eve_in = node_eve.create_gate_cluster(
        String::from("channelIncoming"),
        2,
        des_core::GateType::Input,
        channel,
    );

    node_alice.create_gate_into(
        String::from("channelOutgoing"),
        des_core::GateType::Output,
        channel,
        eve_in[0],
    );
    node_bob.create_gate_into(
        String::from("channelOutgoing"),
        des_core::GateType::Output,
        channel,
        eve_in[1],
    );

    app.create_module(node_alice);
    app.create_module(stack_alice);

    app.create_module(node_bob);
    app.create_module(stack_bob);

    app.create_module(node_eve);
    app.create_module(stack_eve);

    let mut rt = Runtime::new_with(
        app,
        des_core::RuntimeOptions {
            sim_base_unit: des_core::SimTimeUnit::Seconds,
            rng: StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    let pkt = Packet::new(
        (0x00_00_00_ff, 0x00_fe),
        (0x00_00_00_ee, 0x00_fe),
        String::from("PING"),
    );
    let msg = Message::new(2, GATE_NULL, MODULE_NULL, MODULE_NULL, SimTime::ZERO, pkt);

    rt.add_event_in(
        MessageAtGateEvent {
            gate_id: alice_in,
            handled: false,
            message: ManuallyDrop::new(msg),
        },
        0.0.into(),
    );

    rt.run();
}
