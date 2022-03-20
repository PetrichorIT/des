use des::*;
use des_derive::Network;

mod members;
use members::*;

#[derive(Network)]
#[ndl_workspace = "tests/utils"]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A().build_rt();

    println!("{:?}", app.parameters());

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));
    let (_, time, event_count) = rt.run().unwrap();

    assert_eq!(event_count, 83);
    assert_eq!(time, SimTime::from(21.0))
}
