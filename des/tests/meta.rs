use des::prelude::*;
use serial_test::serial;

#[macro_use]
mod common;

struct Parent;
impl_build_named!(Parent);

impl Module for Parent {
    fn new() -> Self {
        Self
    }

    fn at_sim_start(&mut self, _stage: usize) {
        current().set_meta(32u64);
        schedule_in(Message::new().build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, _msg: Message) {
        assert_eq!(current().meta::<u64>(), Some(32));
        assert_eq!(current().meta::<bool>(), None);

        assert_eq!(current().child("child-a").unwrap().meta::<String>(), Some("child-a".to_string()));
        assert_eq!(current().child("child-b").unwrap().meta::<String>(), Some("child-b".to_string()));
    }
}

struct Child;
impl_build_named!(Child);

impl Module for Child {
    fn new() -> Self {
        Self
    }

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

    let mut rt = NetworkApplication::new(());

    let parent = Parent::build_named(ObjectPath::from("root".to_string()), &mut rt);
    let child_a = Child::build_named_with_parent("child-a", parent.clone(), &mut rt);
    let child_b = Child::build_named_with_parent("child-b", parent.clone(), &mut rt);

    rt.register_module(parent);
    rt.register_module(child_a);
    rt.register_module(child_b);


    let rt = Builder::seeded(123).build(rt);

    let res = rt.run();
    let _res = res.unwrap();
}

struct Overrider;
impl_build_named!(Overrider);

impl Module for Overrider {
    fn new() -> Self {
        Self
    }

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

    let mut rt = NetworkApplication::new(());

    let parent = Overrider::build_named(ObjectPath::from("root".to_string()), &mut rt);
    rt.register_module(parent);

    let rt = Builder::seeded(123).build(rt);

    let res = rt.run();
    let _res = res.unwrap();
}