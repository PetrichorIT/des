use des::prelude::*;

mod members;
use members::*;

#[NdlSubsystem("examples/utils")]
#[derive(Debug, Default)]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A::default().build_rt();

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123).quiet());
    let (app, time, p) = rt.run().unwrap();

    let topo = &app.globals_weak().topology;

    assert_eq!(topo.nodes().count(), 4);
    assert_eq!(topo.edges().count(), 14);

    let _ = topo.write_to_svg("examples/utils/graph");

    assert_eq!(p.event_count, 94);
    assert_eq_time!(time, 45.0)
}
