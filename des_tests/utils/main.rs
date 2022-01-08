use des_core::*;
use des_macros::Network;

mod members;
use members::*;
use rand::{prelude::StdRng, SeedableRng};

#[derive(Network)]
#[ndl_workspace = "des_tests/utils"]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A().build_rt();

    let rt = Runtime::new_with(
        app,
        des_core::RuntimeOptions {
            rng: StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    rt.run().unwrap();
}
