use des::{prelude::*, registry};

mod modules;
pub use modules::*;

fn main() {
    // Logger::new().try_set_logger().unwrap();

    let app = Sim::ndl("examples/ptrhell/main.yml", registry![Bob, Alice, Network]).unwrap();

    let rt = Builder::seeded(0x123).build(app.freeze());

    let r = rt.run().assert_no_err();

    assert_eq!(r.profiler.event_count, 7);
    assert_eq!(r.time.as_millis(), 315)
}
