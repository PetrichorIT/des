use des::{prelude::*, registry};
use std::sync::atomic::{AtomicUsize, Ordering};

mod modules;
pub use modules::*;

static MODULE_LEN: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let app = NdlApplication::new("examples/droptest/main.ndl", registry![Network, Bob, Alice])
        .map_err(|e| println!("{e}"))
        .unwrap();

    let rt = Runtime::new_with(NetworkApplication::new(app), RuntimeOptions::seeded(0x123));

    let (app, time, p) = rt.run().unwrap();
    let globals = app.globals();
    drop(app);

    // println!("{:?}", globals);

    drop(globals);

    // // Assume full drop.
    assert_eq!(MODULE_LEN.load(Ordering::SeqCst), 0);

    assert_eq!(p.event_count, 7);
    assert_eq!(time.as_millis(), 387)
}
