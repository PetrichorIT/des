use des::*;
use des_derive::Network;
use rand::SeedableRng;

mod modules;
pub use modules::*;

#[derive(Debug, Network)]
#[ndl_workspace = "tests/ptrhell"]
struct Main();

fn main() {
    let app: NetworkRuntime<Main> = Main().build_rt();

    let rt = Runtime::new_with(
        app,
        des::RuntimeOptions {
            rng: rand::rngs::StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    rt.run().unwrap();
}
