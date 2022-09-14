// #![cfg(feature = "net")]

// use des::{
//     net::{BuildContext, NetworkRuntimeGlobals, __Buildable0},
//     prelude::*,
//     runtime::StandardLogger,
//     util::PtrMut,
// };

// #[NdlModule]
// #[derive(Debug)]
// struct Parent {
//     acummulated_counter: usize,
// }

// impl Module for Parent {
//     fn new() -> Self {
//         Self {
//             acummulated_counter: 0,
//         }
//     }
// }

// #[NdlModule]
// #[derive(Debug)]
// struct Child {
//     counter: usize,
// }

// impl Module for Child {
//     fn new() -> Self {
//         Self { counter: 0 }
//     }
// }

// impl Child {
//     fn inc(&mut self, amount: usize) {
//         self.counter += amount;
//         parent().unwrap().mut_as::<Parent>().acummulated_counter += amount;
//     }
// }

// #[NdlModule]
// #[derive(Debug)]
// struct GrandChild {}

// impl Module for GrandChild {
//     fn new() -> Self {
//         Self {}
//     }
// }

// #[derive(Debug)]
// struct TestCase {
//     parent: ModuleRef,
//     children: Vec<ModuleRef>,
//     grand_children: Vec<ModuleRef>,
// }

// impl TestCase {
//     fn build() -> Self {
//         let mut app = NetworkRuntime::new(());
//         let mut cx = BuildContext::new(&mut app);

//         let mut parent = Parent::build_named(ObjectPath::root_module("Root".to_string()), &mut cx);

//         let mut children = vec![
//             Child::build_named_with_parent("c1", parent.clone(), &mut cx),
//             Child::build_named_with_parent("c2", parent.clone(), &mut cx),
//             Child::build_named_with_parent("c3", parent.clone(), &mut cx),
//         ];

//         let grand_children = vec![
//             GrandChild::build_named_with_parent("left", children[0].clone(), &mut cx),
//             GrandChild::build_named_with_parent("right", children[0].clone(), &mut cx),
//             GrandChild::build_named_with_parent("left", children[1].clone(), &mut cx),
//             GrandChild::build_named_with_parent("right", children[1].clone(), &mut cx),
//             GrandChild::build_named_with_parent("left", children[2].clone(), &mut cx),
//             GrandChild::build_named_with_parent("right", children[2].clone(), &mut cx),
//         ];

//         Self {
//             parent,
//             children,
//             grand_children,
//         }
//     }
// }

// #[test]
// fn test_case_build() {
//     StandardLogger::active(false);

//     let _case = TestCase::build();
// }

// #[test]
// fn test_parent_ptr() {
//     StandardLogger::active(false);

//     let case = TestCase::build();

//     // println!("Parent: {:?}", TypeId::of::<Parent>());
//     // println!("Child: {:?}", TypeId::of::<Child>());
//     // println!("GrandChild: {:?}", TypeId::of::<GrandChild>());

//     // println!("{:?}", case.children[0]);

//     assert_eq!(
//         case.children[0].parent_as::<Parent>().unwrap().id(),
//         case.parent.id()
//     );
//     assert_eq!(case.children[1].parent().unwrap().id(), case.parent.id());
//     assert_eq!(
//         case.children[2].parent_as::<Parent>().unwrap().id(),
//         case.parent.id()
//     );

//     assert_eq!(
//         case.grand_children[0].parent_as::<Child>().unwrap().id(),
//         case.children[0].id()
//     );
//     assert_eq!(
//         case.grand_children[1].parent_as::<Child>().unwrap().id(),
//         case.children[0].id()
//     );
//     assert_eq!(
//         case.grand_children[2].parent().unwrap().id(),
//         case.children[1].id()
//     );
// }

// #[test]
// fn test_parent_mut_ptr() {
//     StandardLogger::active(false);

//     let mut case = TestCase::build();

//     case.children[0].inc(1);
//     case.children[1].inc(2);
//     case.children[2].inc(3);

//     assert_eq!(case.children[0].counter, 1);
//     assert_eq!(case.children[1].counter, 2);
//     assert_eq!(case.children[2].counter, 3);

//     assert_eq!(case.parent.acummulated_counter, 6);
// }
