use des::{prelude::*, processing::TimeDriver};
use des_ndl::{Ndl, registry};

mod members;
use members::*;

#[derive(Debug, Default)]
struct A;

impl Module for A {}

fn main() {
    let mut app = Sim::new(()).with_stack(|| TimeDriver::default()); // NO TOKIO
    app.node(
        "",
        Ndl::from_str(&mut registry![A, Alice, Bob], include_str!("main.yml")).unwrap(),
    )
    .unwrap();

    let rt = app.seeded(0x123).build();

    let r = rt.run().assert_no_err();

    // assert_eq!(tie, 18224.956482853);

    assert_eq!(r.time.as_secs(), 9264);
    assert_eq!(r.app.profiler.event_count, 12_000_901);

    // profile
    //     .write_to("examples/ndl/bench")
    //     .expect("Failed to write bench")
}
