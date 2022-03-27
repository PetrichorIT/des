use des::prelude::*;
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

    assert_eq!(event_count, 94);
    assert_eq!(time, SimTime::from(45.0))
}
