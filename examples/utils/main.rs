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
    let mut app = NetworkRuntime::new(
        NdlApplication::new("examples/utils/main.ndl", registry![A, Alice, Bob])
            .map_err(|e| println!("{e}"))
            .unwrap(),
    );
    app.include_par_file("examples/utils/init.par");

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123).quiet());
    let (app, time, p) = rt.run().unwrap();

    let topo = app.globals().topology.lock().unwrap().clone();

    assert_eq!(topo.nodes().count(), 4 + 1);
    assert_eq!(topo.edges().count(), 14);

    let _ = topo.write_to_svg("examples/utils/graph");

    assert_eq!(p.event_count, 49);
    assert_eq!(time.as_secs(), 83)
}
