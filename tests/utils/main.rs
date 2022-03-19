use des::*;
use des_derive::Network;

mod members;
use members::*;
use rand::{prelude::StdRng, SeedableRng};

#[derive(Network)]
#[ndl_workspace = "tests/utils"]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A().build_rt();

    println!("{:?}", app.parameters());

    let rt = Runtime::new_with(
        app,
        des::RuntimeOptions {
            rng: StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    rt.run().unwrap();
}
