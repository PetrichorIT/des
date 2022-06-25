use des::prelude::*;

mod members;
use members::*;

#[derive(Subsystem)]
#[ndl_workspace = "tests/ndl"]
struct A();

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let options = if args.len() == 3 {
        let n = args[1].parse().unwrap();
        let t = args[2].parse::<f64>().unwrap();
        RuntimeOptions::seeded(0x123).cqueue_options(n, SimTime::from(t))
    } else {
        RuntimeOptions::seeded(0x123)
    };

    let app: NetworkRuntime<A> = A().build_rt();

    let ids: Vec<ModuleRefMut> = (1..=100)
        .map(|n| app.module(|m| m.name() == format!("bob[{}]", n)).unwrap())
        .collect();

    let mut rt = Runtime::new_with(app, options);

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
