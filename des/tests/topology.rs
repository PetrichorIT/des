use des::prelude::*;
use petgraph::algo::tarjan_scc;
use serial_test::serial;

#[test]
#[serial]
fn main() {
    let app = Sim::ndl("tests/ndl/top.yml", Registry::new().with_default_fallback())
        .map_err(|e| println!("{e}"))
        .unwrap();
    let rt = Builder::new().build(app.freeze());
    let app = rt.run().assert_no_err().app;
    let topo = app.globals().topology();

    let connected = dbg!(tarjan_scc(&topo)).len() == 1;
    assert!(!connected);
}

struct Fallback;
impl Module for Fallback {}

#[test]
#[serial]
fn spanned_topology() {
    let mut sim = Sim::new(());

    sim.node("alice", Fallback);
    sim.node("alice.eve", Fallback);
    sim.node("alice.eve.travis", Fallback);
    sim.node("alice.sophie", Fallback);
    sim.node("bob", Fallback);

    sim.gate("alice", "to-eve")
        .connect(sim.gate("alice.eve", "to-alice"));
    sim.gate("alice", "to-sophie")
        .connect(sim.gate("alice.sophie", "to-alice"));
    sim.gate("alice.eve", "to-travis")
        .connect(sim.gate("alice.eve.travis", "to-eve"));
    sim.gate("alice.eve", "to-sophie")
        .connect(sim.gate("alice.sophie", "to-eve"));

    let root = sim.get(&"alice").unwrap();

    let topology = root.spanning_tree();
    assert_eq!(topology.node_count(), 4);
    assert_eq!(topology.edge_count(), 2 * 4);
    assert!(!topology.node_weights().any(|n| n.name() == "bob"));
}
