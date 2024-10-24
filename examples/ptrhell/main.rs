use des::{prelude::*, registry};

mod modules;
pub use modules::*;

fn main() {
    // Logger::new().try_set_logger().unwrap();

    let app = Sim::ndl("examples/ptrhell/main.yml", registry![Bob, Alice, Network]).unwrap();

    let rt = Builder::seeded(0x123).build(app);

    let (_, time, p) = rt.run().unwrap();

    assert_eq!(p.event_count, 6);
    assert_eq!(time.as_millis(), 393)
}
