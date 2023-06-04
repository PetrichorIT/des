use des::{prelude::*, registry};

mod modules;
pub use modules::*;

fn main() {
    // Logger::new().try_set_logger().unwrap();

    let app = NetworkApplication::new(
        NdlApplication::new("examples/ptrhell/main.ndl", registry![Bob, Alice, Network]).unwrap(),
    );

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    let (_, time, p) = rt.run().unwrap();

    assert_eq!(p.event_count, 7);
    assert_eq!(time.as_millis(), 387)
}
