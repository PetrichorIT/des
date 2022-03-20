use des::*;
use des_derive::Network;

mod members;
use members::*;

#[derive(Network)]
#[ndl_workspace = "tests/ndl"]
struct A();

fn main() {
    let app: NetworkRuntime<A> = A().build_rt();

    let ids: Vec<ModuleRef> = (1..=100)
        .map(|n| app.module(|m| m.name() == format!("bob{}", n)).unwrap())
        .collect();

    let mut rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));

    for id in ids {
        let msg = Message::new(
            0,
            0xff,
            GateId::NULL,
            ModuleId::NULL,
            ModuleId::NULL,
            SimTime::now(),
            String::from("Init"),
        );

        let arr_time = SimTime::ZERO;

        rt.handle_message_on(id, msg, arr_time);
    }

    rt.run().unwrap();
}