use des::{prelude::*, registry};

mod members;
use members::*;

#[derive(Debug, Default)]
struct A;

impl Module for A {}

fn main() {
    let app = Sim::ndl("examples/ndl/main.yml", registry![A, Alice, Bob]).unwrap();

    let rt = Builder::seeded(0x123).build(app.freeze());

    let r = rt.run().unwrap_no_err();

    // assert_eq!(tie, 18224.956482853);

    assert_eq!(r.time.as_secs(), 9264);
    assert_eq!(r.profiler.event_count, 12_000_901);

    // profile
    //     .write_to("examples/ndl/bench")
    //     .expect("Failed to write bench")
}
