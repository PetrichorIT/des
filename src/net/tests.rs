#[test]
fn it_works() {
    use super::*;
    use crate::{Runtime, RuntimeOptions, SimTimeUnit};
    use rand::{prelude::StdRng, SeedableRng};

    struct A();

    let mut app: NetworkRuntime<A> = NetworkRuntime::new(A());

    let mut alice = Module::new();
    let mut bob = Module::new();

    app.channels.push(Channel::new(
        GATE_NULL,
        GATE_NULL,
        ChannelMetrics {
            bitrate: 5_000_000,
            latency: 0.1.into(),
            jitter: 0.0.into(),
        },
    ));

    let channel = &app.channels[0];

    // bob ...... alice
    // g4 -> g3 -> g2 -> g1;

    let g1 = alice.create_gate(String::from("netIn"), GateType::Input, channel);
    let g2 = alice.create_gate_into(String::from("netIn"), GateType::Input, channel, g1);
    let g3 = bob.create_gate_into(String::from("netOut"), GateType::Output, channel, g2);
    let g4 = bob.create_gate_into(String::from("netOut"), GateType::Output, channel, g3);

    app.modules.push(alice);
    app.modules.push(bob);

    let rt_options = RuntimeOptions {
        sim_base_unit: SimTimeUnit::Seconds,
        max_itr: !0,
        rng: StdRng::seed_from_u64(0x56123),
    };
    let mut rt = Runtime::new_with(app, rt_options);

    let msg = Message::new(
        &mut rt,
        1,
        GATE_NULL,
        MODULE_NULL,
        MODULE_NULL,
        SimTime::now(),
        42,
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
