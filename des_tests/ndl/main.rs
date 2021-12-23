use std::mem::ManuallyDrop;

use des_core::*;

mod members;
use members::*;
use rand::{prelude::StdRng, SeedableRng};

struct A();

fn main() {
    let mut app = NetworkRuntime::new(A());

    let bob = Bob::build_named("bob", &mut app);

    // let bob = Bob::build(Box::new(Bob::named("bob".to_owned())), &mut app);
    let bob = app.create_module(bob);
    let bob_id = bob.id();

    let mut rt = Runtime::new_with(
        app,
        des_core::RuntimeOptions {
            sim_base_unit: des_core::SimTimeUnit::Seconds,
            rng: StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    let msg = Message::new(
        0,
        GATE_NULL,
        MODULE_NULL,
        MODULE_NULL,
        SimTime::now(),
        String::from("Init"),
    );

    rt.add_event_in(
        HandleMessageEvent {
            module_id: bob_id,
            handled: false,
            message: ManuallyDrop::new(msg),
        },
        0.0.into(),
    );

    rt.run();
}
