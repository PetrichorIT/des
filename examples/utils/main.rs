use std::io::Write;

use des::{prelude::*, registry};

mod members;
use members::*;

#[derive(Debug, Default)]
struct A;
impl Module for A {}

fn main() -> std::io::Result<()> {
    let mut app = Sim::ndl("examples/utils/main.yml", registry![A, Alice, Bob])
        .map_err(|e| println!("{e}"))
        .unwrap();
    app.include_par_file("examples/utils/init.par.yml").unwrap();

    let rt = Builder::seeded(0x123).quiet().build(app);
    let (app, time, p) = rt.run().unwrap();

    let topo = app.globals().topology.lock().unwrap().clone();

    assert_eq!(topo.nodes().len(), 4 + 1);
    assert_eq!(topo.edges().count(), 2 * 14);

    std::fs::File::create("examples/utils/graph.svg")?.write_all(topo.as_svg()?.as_bytes())?;

    assert_eq!(p.event_count, 30);
    assert_eq!(time.as_secs(), 50);

    Ok(())
}
