use des::prelude::*;
use des_derive::Network;

mod modules;
pub use modules::*;

#[derive(Debug, Network)]
#[ndl_workspace = "tests/ptrhell"]
struct Main();

fn main() {
    let app: NetworkRuntime<Main> = Main().build_rt();

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    let (_, time, event_count) = rt.run().unwrap();

    assert_eq!(event_count, 9);
    assert_eq!(time, SimTime::from(0.28533015283430085))
}
