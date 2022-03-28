use des::{net::Topology, prelude::*};
use des_derive::Network;

mod members;
use members::*;

#[derive(Network)]
#[ndl_workspace = "tests/utils"]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A().build_rt();

    println!("{:?}", app.globals().parameters);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));
    let (app, time, event_count) = rt.run().unwrap();

    let topo = &app.globals().topology;

    assert_eq!(topo.nodes().len(), 4);
    assert_eq!(topo.edges().len(), 4);
    assert_eq!(
        topo.edges()
            .iter()
            .map(|outgoing| outgoing.0.len())
            .fold(0, |acc, c| acc + c),
        14
    );

    let _ = write_graph(topo);

    assert_eq!(event_count, 94);
    assert_eq!(time, SimTime::from(45.0))
}

fn write_graph(topo: &Topology) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;
    let str = topo.dot_output();
    let mut file = File::create("tests/utils/graph.dot")?;
    write!(file, "{}", str)?;

    let output = Command::new("dot")
        .arg("-Tsvg")
        .arg("tests/utils/graph.dot")
        .output()?;

    let mut file = File::create("tests/utils/graph.svg")?;
    write!(file, "{}", String::from_utf8_lossy(&output.stdout))?;

    Ok(())
}
