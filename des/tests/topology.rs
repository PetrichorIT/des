#![cfg(feature = "ndl")]
use des::prelude::*;

#[test]
fn main() {
    let app = Sim::ndl(
        "tests/ndl/small_network/main.ndl",
        Registry::new().with_default_fallback(),
    )
    .map_err(|e| println!("{e}"))
    .unwrap();
    let rt = Builder::new().build(app);
    let app = rt.run().into_app();
    let mut topo = app.globals().topology.lock().unwrap().clone();

    let dj = topo.dijkstra("node[1]".into());
    assert_eq!(dj.get(&"node[1]".into()), None);

    topo.filter_nodes(|n| n.module.name() != "node[2]");
    topo.map_costs(|edge| edge.cost * 2.0);
    topo.filter_edges(|_| true);
    assert_eq!(topo.edges(), 9);

    // 4 nodes, router, debugger, main
    assert_eq!(topo.nodes().len(), 7);
    assert_eq!(topo.nodes().into_iter().filter(|n| n.alive).count(), 6);

    let i = topo
        .nodes()
        .into_iter()
        .position(|n| n.module.name() == "router")
        .unwrap();

    let j = topo
        .nodes()
        .into_iter()
        .position(|n| n.module.name() == "debugger")
        .unwrap();

    assert!(topo
        .edges_for(i)
        .iter()
        .any(|edge| edge.dst.1 == j && edge.src.0.name() == "debug"));

    let _ = topo.write_to_svg("tests/topology");
}
