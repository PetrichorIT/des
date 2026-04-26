use des::Sim;
use des_ndl::{Registry, SimExt};
use petgraph::algo::tarjan_scc;
use serial_test::serial;

#[test]
#[serial]
fn main() {
    let app = Sim::ndl("tests/ndl/top.yml", Registry::new().with_default_fallback())
        .map_err(|e| println!("{e}"))
        .unwrap();
    let rt = app.build();
    let app = rt.run().assert_no_err().app;
    let topo = app.globals().topology();

    let connected = tarjan_scc(&topo).len() == 1;
    assert!(!connected);
}
