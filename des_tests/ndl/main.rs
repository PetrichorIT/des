use des_core::*;
use des_macros::Network;

mod members;
use members::*;
use rand::{prelude::StdRng, SeedableRng};

#[derive(Network)]
#[ndl_workspace = "des_tests/ndl"]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A().build_rt();

    let ids: Vec<ModuleId> = (1..=100)
        .map(|n| {
            app.module(|m| m.name().unwrap() == &format!("bob{}", n))
                .unwrap()
                .id()
        })
        .collect();

    let mut rt = Runtime::new_with(
        app,
        des_core::RuntimeOptions {
            sim_base_unit: des_core::SimTimeUnit::Seconds,
            rng: StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    for id in ids {
        let msg = Message::new(
            0xff,
            GATE_NULL,
            MODULE_NULL,
            MODULE_NULL,
            SimTime::now(),
            String::from("Init"),
        );

        let arr_time = id.raw() as f64 / 1000.0;

        rt.handle_message_on(id, msg, arr_time.into());
    }

    rt.run().unwrap();
}
