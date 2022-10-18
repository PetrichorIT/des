use des::prelude::*;

mod members;
use members::*;

#[NdlSubsystem("examples/ndl")]
#[derive(Debug, Default)]
struct A();

fn main() {
    let options = RuntimeOptions::seeded(0x123).include_env();

    let app = A::default().build_rt();

    let ids: Vec<ModuleRef> = (1..=100)
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

    let (_, time, profile) = rt.run().unwrap();

    // assert_eq!(tie, 18224.956482853);

    assert_eq!(time.as_secs(), 20460);
    assert_eq!(profile.event_count, 40_001_301);

    // profile
    //     .write_to("examples/ndl/bench")
    //     .expect("Failed to write bench")
}
