use des::prelude::*;
use des_ndl::{SimExt, registry};

mod modules;
pub use modules::*;

fn main() {
    let app = Sim::ndl("examples/ptrhell/main.yml", registry![Bob, Alice, Network]).unwrap();

    let rt = app.seeded(0x123).build();

    let r = rt.run().assert_no_err();

    assert_eq!(r.app.profiler.event_count, 7);
    assert_eq!(r.time.as_millis(), 315)
}
