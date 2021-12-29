use std::mem::ManuallyDrop;

use des_core::*;
use des_macros::Network;

mod members;
use members::*;
use rand::{prelude::StdRng, SeedableRng};

#[derive(Network)]
#[ndl_workspace = "ndl"]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A().build_rt();

    let bob_id = app.module(|m| m.name().unwrap() == "bob1").unwrap().id();

    let mut rt = Runtime::new_with(
        app,
        des_core::RuntimeOptions {
            sim_base_unit: des_core::SimTimeUnit::Seconds,
            rng: StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    let msg = Message::new(
        0xff,
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
