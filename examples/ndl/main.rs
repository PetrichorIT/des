use des::{prelude::*, registry};

mod members;
use members::*;

#[derive(Debug, Default)]
struct A;

impl Module for A {}

fn main() {
    let app = Sim::ndl("examples/ndl/main.ndl", registry![A, Alice, Bob]).unwrap();

    let rt = Builder::seeded(0x123).build(app);

    let (_, time, profile) = rt.run().unwrap();

    // assert_eq!(tie, 18224.956482853);

    assert_eq!(time.as_secs(), 12278);
    assert_eq!(profile.event_count, 18_001_000);

    // profile
    //     .write_to("examples/ndl/bench")
    //     .expect("Failed to write bench")
}
