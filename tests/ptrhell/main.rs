use des::prelude::*;

mod modules;
pub use modules::*;

#[NdlSubsystem("tests/ptrhell")]
#[derive(Debug, Default)]
struct Main();

fn main() {
    let app: NetworkRuntime<Main> = Main::default().build_rt();

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    let (_, time, event_count) = rt.run().unwrap();

    assert_eq!(event_count, 9);
    assert_eq!(time, SimTime::from(0.285330151))
}
