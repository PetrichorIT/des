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
    let mut app = NetworkApplication::new(
        NdlApplication::new("examples/utils/main.ndl", registry![A, Alice, Bob])
            .map_err(|e| println!("{e}"))
            .unwrap(),
    );
    app.include_par_file("examples/utils/init.par");

    let rt = Builder::seeded(0x123).quiet().build(app);
    let (app, time, p) = rt.run().unwrap();

    let topo = app.globals().topology.lock().unwrap().clone();

    assert_eq!(topo.nodes().len(), 4 + 1);
    assert_eq!(topo.edges(), 14);

    let _ = topo.write_to_svg("examples/utils/graph");

    assert_eq!(p.event_count, 48);
    assert_eq!(time.as_secs(), 83)
}
