use des::*;
use des_derive::Network;

mod modules;
pub use modules::*;

#[derive(Debug, Network)]
#[ndl_workspace = "tests/ptrhell"]
struct Main();

fn main() {
    let app: NetworkRuntime<Main> = Main().build_rt();

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    rt.run().unwrap();
}
