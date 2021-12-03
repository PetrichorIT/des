#[test]
fn it_works() {
    use super::*;
    use crate::sim_time_fmt;
    use crate::{Runtime, RuntimeOptions, SimTimeUnit};
    use rand::{prelude::StdRng, SeedableRng};

    struct A();

    struct Alice(ModuleCore);

    impl Module for Alice {
        fn module_core(&self) -> &ModuleCore {
            &self.0
        }

        fn module_core_mut(&mut self) -> &mut ModuleCore {
            &mut self.0
        }

        fn handle_message(&mut self, msg: Message) {
            info!(target: "Alice", "Received at {}: message #{:?} content: {}", sim_time_fmt(),msg.id(), msg.extract_content::<String>());

            self.send(
                Message::new(
                    1,
                    GATE_NULL,
                    self.id(),
                    43,
                    SimTime::ZERO,
                    String::from("Pong"),
                ),
                ("netOut", 0),
            )
        }
    }

    struct Bob(ModuleCore);

    impl Module for Bob {
        fn module_core(&self) -> &ModuleCore {
            &self.0
        }

        fn module_core_mut(&mut self) -> &mut ModuleCore {
            &mut self.0
        }

        fn handle_message(&mut self, msg: Message) {
            info!(target: "Bob", "Received at {}: message #{:?} content: {}", sim_time_fmt(),msg.id(), msg.extract_content::<String>());
        }
    }

    let mut alice = Alice(ModuleCore::new());
    let mut bob = Bob(ModuleCore::new());

    let mut app: NetworkRuntime<A> = NetworkRuntime::new(A());

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

    let r1 = bob.create_gate(String::from("netIn"), GateType::Input, channel);
    let r2 = bob.create_gate_into(String::from("netIn"), GateType::Input, channel, r1);
    let r3 = alice.create_gate_into(String::from("netOut"), GateType::Output, channel, r2);
    let _r4 = alice.create_gate_into(String::from("netOut"), GateType::Output, channel, r3);

    app.modules.push(Box::new(alice));
    app.modules.push(Box::new(bob));

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
