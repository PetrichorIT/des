use des_core::*;
use rand::{prelude::StdRng, SeedableRng};

mod alice;
mod bob;

struct Application();

fn main() {
    let mut alice = Mrc::new(alice::Alice(ModuleCore::new()));
    let mut bob = Mrc::new(bob::Bob(ModuleCore::new()));

    bob.add_child(&mut *alice);

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

    let mut rt = Runtime::new_with(
        app,
        RuntimeOptions {
            max_itr: 200,
            rng: StdRng::seed_from_u64(0x56123),
        },
    );

    let msg = Message::new(
        0,
        1,
        GateId::NULL,
        ModuleId::NULL,
        ModuleId::NULL,
        SimTime::now(),
        String::from("Ping"),
    );

    rt.add_message_onto(g4, msg, 1.0.into());

    rt.run();
}
