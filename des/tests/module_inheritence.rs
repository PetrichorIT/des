#![cfg(feature = "net")]

use des::{net::NetworkRuntimeGlobals, prelude::*, runtime::StandardLogger, util::PtrMut};

macro_rules! auto_impl_static {
    ($ident: ident) => {
        impl std::ops::Deref for $ident {
            type Target = ModuleCore;
            fn deref(&self) -> &Self::Target {
                &self.core
            }
        }

        impl std::ops::DerefMut for $ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.core
            }
        }
    };
}

#[derive(Debug)]
struct Parent {
    core: ModuleCore,
    acummulated_counter: usize,
}

impl NameableModule for Parent {
    fn named(core: ModuleCore) -> Self {
        Self {
            core,
            acummulated_counter: 0,
        }
    }
}

auto_impl_static!(Parent);

#[derive(Debug)]
struct Child {
    core: ModuleCore,
    counter: usize,
}

impl NameableModule for Child {
    fn named(core: ModuleCore) -> Self {
        Self { core, counter: 0 }
    }
}

impl Child {
    fn inc(&mut self, amount: usize) {
        self.counter += amount;
        self.parent_mut_as::<Parent>().unwrap().acummulated_counter += amount;
    }
}

auto_impl_static!(Child);

#[derive(Debug)]
struct GrandChild {
    core: ModuleCore,
}

impl NameableModule for GrandChild {
    fn named(core: ModuleCore) -> Self {
        Self { core }
    }
}

auto_impl_static!(GrandChild);

#[derive(Debug)]
struct TestCase {
    parent: PtrMut<Parent>,
    children: Vec<PtrMut<Child>>,
    grand_children: Vec<PtrMut<GrandChild>>,
}

impl TestCase {
    fn build() -> Self {
        let core = ModuleCore::new_with(
            ObjectPath::root_module("Root".into()),
            PtrWeakConst::from_strong(&PtrConst::new(NetworkRuntimeGlobals::new())),
        );

        let mut parent = Parent::named_root(core);

        let mut children = vec![
            Child::named_with_parent("c1", &mut parent),
            Child::named_with_parent("c2", &mut parent),
            Child::named_with_parent("c3", &mut parent),
        ];

        let grand_children = vec![
            GrandChild::named_with_parent("left", &mut children[0]),
            GrandChild::named_with_parent("right", &mut children[0]),
            GrandChild::named_with_parent("left", &mut children[1]),
            GrandChild::named_with_parent("right", &mut children[1]),
            GrandChild::named_with_parent("left", &mut children[2]),
            GrandChild::named_with_parent("right", &mut children[2]),
        ];

        Self {
            parent,
            children,
            grand_children,
        }
    }
}

#[test]
fn test_case_build() {
    StandardLogger::active(false);

    let _case = TestCase::build();
}

#[test]
fn test_parent_ptr() {
    StandardLogger::active(false);

    let case = TestCase::build();

    // println!("Parent: {:?}", TypeId::of::<Parent>());
    // println!("Child: {:?}", TypeId::of::<Child>());
    // println!("GrandChild: {:?}", TypeId::of::<GrandChild>());

    // println!("{:?}", case.children[0]);

    assert_eq!(
        case.children[0].parent_as::<Parent>().unwrap().id(),
        case.parent.id()
    );
    assert_eq!(case.children[1].parent().unwrap().id(), case.parent.id());
    assert_eq!(
        case.children[2].parent_as::<Parent>().unwrap().id(),
        case.parent.id()
    );

    assert_eq!(
        case.grand_children[0].parent_as::<Child>().unwrap().id(),
        case.children[0].id()
    );
    assert_eq!(
        case.grand_children[1].parent_as::<Child>().unwrap().id(),
        case.children[0].id()
    );
    assert_eq!(
        case.grand_children[2].parent().unwrap().id(),
        case.children[1].id()
    );
}

#[test]
fn test_parent_mut_ptr() {
    StandardLogger::active(false);

    let mut case = TestCase::build();

    case.children[0].inc(1);
    case.children[1].inc(2);
    case.children[2].inc(3);

    assert_eq!(case.children[0].counter, 1);
    assert_eq!(case.children[1].counter, 2);
    assert_eq!(case.children[2].counter, 3);

    assert_eq!(case.parent.acummulated_counter, 6);
}
