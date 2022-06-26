use des::prelude::*;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

mod modules;
pub use modules::*;

lazy_static! {
    static ref MODULE_LEN: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
}

#[NdlSubsystem("tests/droptest")]
#[derive(Debug, Default)]
struct Main();

fn main() {
    let app: NetworkRuntime<Main> = Main::default().build_rt();

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    let (app, time, event_count) = rt.run().unwrap();
    let globals = app.globals();

    drop(app);

    // println!("{:?}", globals);

    // for m in globals.topology.nodes() {
    //     println!("> {:?}", m)
    // }

    drop(globals);
    // Assume full drop.
    assert_eq!(*MODULE_LEN.lock().unwrap(), 0);

    assert_eq!(event_count, 9);
    assert_eq!(time, 0.285330151)
}
