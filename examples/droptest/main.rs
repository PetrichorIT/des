use des::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

mod modules;
pub use modules::*;

static MODULE_LEN: AtomicUsize = AtomicUsize::new(0);

#[NdlSubsystem("examples/droptest")]
#[derive(Debug, Default)]
struct Main();

fn main() {
    let app = Main::default().build_rt();

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    let (app, time, p) = rt.run().unwrap();
    let globals = app.globals();
    drop(app);

    // println!("{:?}", globals);

    drop(globals);

    // // Assume full drop.
    assert_eq!(MODULE_LEN.load(Ordering::SeqCst), 0);

    assert_eq!(p.event_count, 9);
    assert_eq!(time.as_millis(), 387)
}
