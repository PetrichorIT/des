use des::{prelude::*, registry};

mod members;
use members::*;

#[derive(Debug, Default)]
struct A;

impl Module for A {
    fn new() -> Self {
        Self
    }
}

fn main() {
    let options = RuntimeOptions::seeded(0x123).include_env();

    let app = NetworkRuntime::new(
        NdlApplication::new("examples/ndl/main.ndl", registry![A, Alice, Bob]).unwrap(),
    );

    let rt = Runtime::new_with(app, options);

    let (_, time, profile) = rt.run().unwrap();

    // assert_eq!(tie, 18224.956482853);

    assert_eq!(time.as_secs(), 12279);
    assert_eq!(profile.event_count, 18_001_001);

    // profile
    //     .write_to("examples/ndl/bench")
    //     .expect("Failed to write bench")
}
