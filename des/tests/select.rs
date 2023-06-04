#![cfg(feature = "macros")]
use des::prelude::*;
use tokio::spawn;

#[macro_use]
mod common;

struct Main;
impl_build_named!(Main);

#[async_trait::async_trait]
impl AsyncModule for Main {
    fn new() -> Main {
        Main
    }

    async fn at_sim_start(&mut self, _: usize) {
        spawn(async move {
            des::select! {
                // Note that this test may change its result, if another call to the RNG
                // is added before the simulation reaches this point.
                // Thus this test may change, however, it should only change if RNG access changes
                _ = std::future::ready(()) => {
                    panic!("This branch should never be chossen, RNG will choose 2")
                },
                _ = std::future::ready(()) => {
                    // Expected result
                },
            }
        });
    }
}

#[test]
fn deterministic_branching() {
    // Since the invalid behaviour is indetermistic.,
    // check multiple iterations
    for _ in 0..100 {
        let mut rt = NetworkApplication::new(());

        let module = Main::build_named(ObjectPath::from("root"), &mut rt);
        rt.register_module(module);
        let rt = Runtime::new_with(rt, RuntimeOptions::seeded(123));
        let v = rt.run();
        assert!(matches!(v, RuntimeResult::EmptySimulation { .. }));
    }
}
