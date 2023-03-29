#![cfg(feature = "ndl")]
use des::{prelude::*, registry};

macro_rules! module {
    ($($i:ident),*) => {
        $(struct $i;
        impl Module for $i {
            fn new() -> Self {
                Self
            }
        })*
    };
}

module!(Node, Debugger, Router, Main);

#[test]
fn topology_load() {
    let app = NdlApplication::new(
        "tests/ndl/small_network/main.ndl",
        registry![Node, Debugger, Router, Main],
    )
    .map_err(|e| println!("{e}"))
    .unwrap();
    let rt = Runtime::new(NetworkApplication::new(app));
    let app = rt.run().into_app();
    let mut topo = app.globals().topology.lock().unwrap().clone();

    topo.filter_nodes(|n| n.module.name() != "node[2]");

    // 4 nodes, router, debugger, main
    assert_eq!(topo.nodes().len(), 7);
    assert_eq!(topo.nodes().into_iter().filter(|n| n.alive).count(), 6);

    let (i, _) = topo
        .nodes()
        .into_iter()
        .enumerate()
        .find(|(_, n)| n.module.name() == "router")
        .unwrap();

    let (j, _) = topo
        .nodes()
        .into_iter()
        .enumerate()
        .find(|(_, n)| n.module.name() == "debugger")
        .unwrap();

    assert!(topo
        .edges_for(i)
        .iter()
        .any(|edge| edge.dst_id == j && edge.src.name() == "debug"))
}
