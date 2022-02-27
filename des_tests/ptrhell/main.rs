use des_core::*;
use des_macros::Network;
use rand::SeedableRng;

mod modules;
pub use modules::*;

#[derive(Debug, Network)]
#[ndl_workspace = "des_tests/ptrhell"]
struct Main();

fn main() {
    let app: NetworkRuntime<Main> = Main().build_rt();

    let rt = Runtime::new_with(
        app,
        des_core::RuntimeOptions {
            rng: rand::rngs::StdRng::seed_from_u64(0x123),
            max_itr: !0,
        },
    );

    rt.run().unwrap();
}
