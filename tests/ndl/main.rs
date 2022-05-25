use des::prelude::*;

mod members;
use members::*;

#[derive(Network)]
#[ndl_workspace = "tests/ndl"]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A().build_rt();

    let ids: Vec<ModuleRefMut> = (1..=100)
        .map(|n| app.module(|m| m.name() == format!("bob[{}]", n)).unwrap())
        .collect();

    let mut rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    for id in ids {
        let msg = Message::new()
            .kind(0xff)
            .content("Init".to_string())
            .build();

        let arr_time = SimTime::ZERO;

        rt.handle_message_on(id, msg, arr_time);
    }

    let (_, _time, event_count) = rt.run().unwrap();

    // assert_eq!(tie, 18224.956482853);
    // assert_eq!(time, SimTime::from(18240.01467112172));
    assert_eq!(event_count, 40_001_301);
}
