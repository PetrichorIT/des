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

    let bob1_id = app.module(|m| m.name().unwrap() == "bob1").unwrap().id();
    let bob2_id = app.module(|m| m.name().unwrap() == "bob2").unwrap().id();
    let bob3_id = app.module(|m| m.name().unwrap() == "bob3").unwrap().id();
    let bob4_id = app.module(|m| m.name().unwrap() == "bob4").unwrap().id();
    let bob5_id = app.module(|m| m.name().unwrap() == "bob5").unwrap().id();

    let mut rt = Runtime::new_with(
        app,
        des_core::RuntimeOptions {
            sim_base_unit: des_core::SimTimeUnit::Seconds,
            rng: StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    for id in vec![bob1_id, bob2_id, bob3_id, bob4_id, bob5_id] {
        let msg = Message::new(
            0xff,
            GATE_NULL,
            MODULE_NULL,
            MODULE_NULL,
            SimTime::now(),
            String::from("Init"),
        );

        let arr_time = id.0 as f64 / 1000.0;

        rt.add_event_in(
            HandleMessageEvent {
                module_id: id,
                handled: false,
                message: ManuallyDrop::new(msg),
            },
            arr_time.into(),
        );
    }

    let (_, end_time) = rt.run().unwrap();

    println!(
        "Sim finished {}",
        SimTimeUnit::fmt_compact(end_time, SimTimeUnit::Seconds)
    );
}
