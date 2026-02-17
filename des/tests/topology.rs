use des::prelude::*;
use serial_test::serial;

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
