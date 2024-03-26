#![cfg(feature = "ndl")]
use std::{fs, io::Write};

use des::prelude::*;

#[test]
fn main() {
    let app = Sim::ndl("tests/ndl/top.ndl", Registry::new().with_default_fallback())
        .map_err(|e| println!("{e}"))
        .unwrap();
    let rt = Builder::new().build(app);
    let app = rt.run().into_app();
    let mut topo = app
        .globals()
        .topology
        .lock()
        .unwrap()
        .clone()
        .with_edge_cost_attachment()
        .with_node_connectivity_attachment();

    assert!(topo.bidirectional());
    assert!(!topo.connected());

    let dj = topo.dijkstra("node[1]");
    assert_eq!(dj.get(&"node[1]".into()), None);

    topo.filter_nodes(|n| n.module().name() != "node[2]");

    assert_eq!(topo.edges().count(), 10);

    // 4 nodes, router, debugger, main + distant
    assert_eq!(topo.nodes().len(), 8);

    topo.filter_nodes(|node| node.degree > 0);
    assert_eq!(topo.nodes().len(), 6);
    assert_eq!(topo.edges().count(), 10);

    assert!(topo
        .edges_for("router")
        .any(|edge| edge.to.gate().owner().path().as_str() == "debugger"));

    fs::File::create("tests/topology.svg")
        .unwrap()
        .write_all(topo.as_svg().unwrap().as_bytes())
        .unwrap();
}
