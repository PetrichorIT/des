use des::prelude::*;

mod modules;
pub use modules::*;

#[NdlSubsystem("examples/ptrhell")]
#[derive(Debug, Default)]
struct Main();

fn main() {
    Logger::new().try_set_logger().unwrap();

    let app = Main::default().build_rt();

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    let (_, time, p) = rt.run().unwrap();

    assert_eq!(p.event_count, 9);
    assert_eq!(time.as_millis(), 387)
}
