use des::prelude::*;
use serial_test::serial;

struct Parent;

impl Module for Parent {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_meta(32u64);
        schedule_in(Message::new().build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, _msg: Message) {
        assert_eq!(current().meta::<u64>(), Some(32));
        assert_eq!(current().meta::<bool>(), None);

        assert_eq!(
            current().child("a").unwrap().meta::<String>(),
            Some("a".to_string())
        );
        assert_eq!(
            current().child("b").unwrap().meta::<String>(),
            Some("b".to_string())
        );
    }
}

struct Child;

impl Module for Child {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_meta(current().name());
        schedule_in(Message::new().build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, _msg: Message) {
        assert_eq!(current().parent().unwrap().meta::<u64>(), Some(32));
    }
}

#[test]
#[serial]
fn read_meta_through_tree() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = Sim::new(());

    rt.node("root", Parent);
    rt.node("root.a", Child);
    rt.node("root.b", Child);

    let rt = Builder::seeded(123).build(rt);

    let res = rt.run();
    let _res = res.unwrap();
}

struct Overrider;
impl Module for Overrider {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_meta(32u64);
        schedule_in(Message::new().build(), Duration::from_secs(1));
        current().set_meta(64u64);
    }

    fn handle_message(&mut self, _msg: Message) {
        assert_eq!(current().meta::<u64>(), Some(64));
    }
}

#[test]
#[serial]
fn meta_override_previous_value() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = Sim::new(());
    rt.node("root", Overrider);

    let rt = Builder::seeded(123).build(rt);

    let res = rt.run();
    let _res = res.unwrap();
}
