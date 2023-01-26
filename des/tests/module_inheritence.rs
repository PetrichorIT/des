#![cfg(feature = "net")]

use des::{
    net::{BuildContext, __Buildable0},
    prelude::*,
};
use serial_test::serial;

#[NdlModule]
#[derive(Debug)]
struct Parent {
    acummulated_counter: usize,
}

impl Module for Parent {
    fn new() -> Self {
        Self {
            acummulated_counter: 0,
        }
    }
}

#[NdlModule]
#[derive(Debug)]
struct Child {
    counter: usize,
}

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

#[NdlModule]
#[derive(Debug)]
struct GrandChild {}

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
        let mut app = NetworkRuntime::new(());
        let mut cx = BuildContext::new(&mut app);

        let parent = Parent::build_named(ObjectPath::root_module("Root".to_string()), &mut cx);

        let children = vec![
            Child::build_named_with_parent("c1", parent.clone(), &mut cx),
            Child::build_named_with_parent("c2", parent.clone(), &mut cx),
            Child::build_named_with_parent("c3", parent.clone(), &mut cx),
        ];

        let grand_children = vec![
            GrandChild::build_named_with_parent("left", children[0].clone(), &mut cx),
            GrandChild::build_named_with_parent("right", children[0].clone(), &mut cx),
            GrandChild::build_named_with_parent("left", children[1].clone(), &mut cx),
            GrandChild::build_named_with_parent("right", children[1].clone(), &mut cx),
            GrandChild::build_named_with_parent("left", children[2].clone(), &mut cx),
            GrandChild::build_named_with_parent("right", children[2].clone(), &mut cx),
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
