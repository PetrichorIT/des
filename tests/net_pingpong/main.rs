use std::mem::ManuallyDrop;

use dse::*;
use rand::{prelude::StdRng, SeedableRng};

mod alice;
mod bob;

struct Application();

fn main() {
    let mut alice = alice::Alice(ModuleCore::new());
    let mut bob = bob::Bob(ModuleCore::new());

    let mut app = NetworkRuntime::new(Application());

    let channel = app.create_channel(ChannelMetrics {
        bitrate: 5_000_000,
        latency: 0.1.into(),
        jitter: 0.0.into(),
    });

    let g1 = alice.create_gate(String::from("netIn"), GateType::Input, channel);
    let g2 = alice.create_gate_into(String::from("netIn"), GateType::Input, channel, g1);
    let g3 = bob.create_gate_into(String::from("netOut"), GateType::Output, channel, g2);
    let g4 = bob.create_gate_into(String::from("netOut"), GateType::Output, channel, g3);

    let r1 = bob.create_gate(String::from("netIn"), GateType::Input, channel);
    let r2 = bob.create_gate_into(String::from("netIn"), GateType::Input, channel, r1);
    let r3 = alice.create_gate_into(String::from("netOut"), GateType::Output, channel, r2);
    let _r4 = alice.create_gate_into(String::from("netOut"), GateType::Output, channel, r3);

    app.create_module(Box::new(alice));
    app.create_module(Box::new(bob));

    let mut rt = Runtime::new_with(
        app,
        RuntimeOptions {
            sim_base_unit: SimTimeUnit::Seconds,
            max_itr: 200,
            rng: StdRng::seed_from_u64(0x56123),
        },
    );

    let msg = Message::new(
        1,
        GATE_NULL,
        MODULE_NULL,
        MODULE_NULL,
        SimTime::now(),
        String::from("Ping"),
    );

    rt.add_event_in(
        MessageAtGateEvent {
            gate_id: g4,
            handled: false,
            message: ManuallyDrop::new(msg),
        },
        1.0.into(),
    );

    rt.run();
}
