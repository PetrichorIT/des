#![cfg(feature = "net")]

use des::prelude::*;
use serial_test::serial;

#[macro_use]
mod common;

#[derive(Debug)]
struct Parent {
    acummulated_counter: usize,
}
impl_build_named!(Parent);

impl Module for Parent {
    fn new() -> Self {
        Self {
            acummulated_counter: 0,
        }
    }
}

#[derive(Debug)]
struct Child {
    counter: usize,
}
impl_build_named!(Child);

impl Module for Child {
    fn new() -> Self {
        Self { counter: 0 }
    }
}

impl Child {
    fn inc(&mut self, amount: usize) {
        self.counter += amount;
        parent().unwrap().as_mut::<Parent>().acummulated_counter += amount;
    }
}

#[derive(Debug)]
struct GrandChild {}
impl_build_named!(GrandChild);

impl Module for GrandChild {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Debug)]
struct TestCase {
    parent: ModuleRef,
    children: Vec<ModuleRef>,
    grand_children: Vec<ModuleRef>,
}

impl TestCase {
    fn build() -> Self {
        let mut app = NetworkApplication::new(());

        let parent = Parent::build_named(ObjectPath::from("Root".to_string()), &mut app);

        let children = vec![
            Child::build_named_with_parent("c1", parent.clone(), &mut app),
            Child::build_named_with_parent("c2", parent.clone(), &mut app),
            Child::build_named_with_parent("c3", parent.clone(), &mut app),
        ];

        let grand_children = vec![
            GrandChild::build_named_with_parent("left", children[0].clone(), &mut app),
            GrandChild::build_named_with_parent("right", children[0].clone(), &mut app),
            GrandChild::build_named_with_parent("left", children[1].clone(), &mut app),
            GrandChild::build_named_with_parent("right", children[1].clone(), &mut app),
            GrandChild::build_named_with_parent("left", children[2].clone(), &mut app),
            GrandChild::build_named_with_parent("right", children[2].clone(), &mut app),
        ];

        Self {
            parent,
            children,
            grand_children,
        }
    }
}

#[test]
#[serial]
fn test_case_build() {
    let _case = TestCase::build();
}

#[test]
#[serial]
fn test_parent_ptr() {
    let case = TestCase::build();

    // println!("Parent: {:?}", TypeId::of::<Parent>());
    // println!("Child: {:?}", TypeId::of::<Child>());
    // println!("GrandChild: {:?}", TypeId::of::<GrandChild>());

    // println!("{:?}", case.children[0]);

    assert_eq!(case.children[0].parent().unwrap().id(), case.parent.id());
    assert_eq!(case.children[1].parent().unwrap().id(), case.parent.id());
    assert_eq!(case.children[2].parent().unwrap().id(), case.parent.id());

    assert_eq!(
        case.grand_children[0].parent().unwrap().id(),
        case.children[0].id()
    );
    assert_eq!(
        case.grand_children[1].parent().unwrap().id(),
        case.children[0].id()
    );
    assert_eq!(
        case.grand_children[2].parent().unwrap().id(),
        case.children[1].id()
    );
}

#[test]
#[serial]
fn test_parent_mut_ptr() {
    let case = TestCase::build();

    // NOTE: inc internally used parent() (glob scope)
    // thus to attach the corret ModuleContext valued use activate
    case.children[0].activate();
    case.children[0].as_mut::<Child>().inc(1);

    case.children[1].activate();
    case.children[1].as_mut::<Child>().inc(2);

    case.children[2].activate();
    case.children[2].as_mut::<Child>().inc(3);

    assert_eq!(case.children[0].as_ref::<Child>().counter, 1);
    assert_eq!(case.children[1].as_ref::<Child>().counter, 2);
    assert_eq!(case.children[2].as_ref::<Child>().counter, 3);

    assert_eq!(case.parent.as_ref::<Parent>().acummulated_counter, 6);
}

struct WillShutdown {
    _state: usize,
}
impl_build_named!(WillShutdown);
impl Module for WillShutdown {
    fn new() -> Self {
        Self { _state: 42 }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        if SimTime::now() == SimTime::ZERO {
            shutdow_and_restart_in(Duration::from_secs_f64(4.5));
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!("This should not happen");
    }
}

struct WillTryToAccess;
impl_build_named!(WillTryToAccess);
impl Module for WillTryToAccess {
    fn new() -> Self {
        Self
    }

    fn at_sim_start(&mut self, _: usize) {
        schedule_in(Message::new().build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, _msg: Message) {
        if SimTime::now().as_secs() > 10 {
            return;
        }

        // first '1'2'3'4 seconds will get denied, other ones not
        let root = module_path().name() == "parent";

        if root {
            let r = child("child");
            if SimTime::now().as_secs() < 5 {
                assert!(r.is_err());
                assert_eq!(
                    r,
                    Err(ModuleReferencingError::CurrentlyInactive(
                        "The child module 'child' of 'parent' is currently shut down, thus cannot be accessed".to_string()
                    ))
                );
            } else {
                assert!(r.is_ok());
            }
        } else {
            let r = parent();
            if SimTime::now().as_secs() < 5 {
                assert!(r.is_err());
                assert_eq!(
                    r,
                    Err(ModuleReferencingError::CurrentlyInactive(
                        "The parent module of 'parent.child' is currently shut down, thus cannot be accessed".to_string()
                    ))
                );
            } else {
                assert!(r.is_ok());
            }
        }

        schedule_in(Message::new().build(), Duration::from_secs(1));
    }
}

#[test]
#[serial]
fn shutdown_modules_cannot_be_accessed_parent() {
    let mut app = NetworkApplication::new(());

    let parent = WillShutdown::build_named(ObjectPath::from("parent"), &mut app);
    let child = WillTryToAccess::build_named_with_parent("child", parent.clone(), &mut app);

    app.register_module(parent);
    app.register_module(child);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let _ = rt.run();
}

#[test]
#[serial]
fn shutdown_modules_cannot_be_accessed_child() {
    let mut app = NetworkApplication::new(());

    let parent = WillTryToAccess::build_named(ObjectPath::from("parent"), &mut app);
    let child = WillShutdown::build_named_with_parent("child", parent.clone(), &mut app);

    app.register_module(parent);
    app.register_module(child);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let _ = rt.run();
}
