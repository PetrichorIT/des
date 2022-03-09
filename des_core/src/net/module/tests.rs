use std::any::TypeId;

use crate::*;

macro_rules! auto_impl_static {
    ($ident: ident) => {
        impl Indexable for $ident {
            type Id = ModuleId;

            fn id(&self) -> Self::Id {
                self.module_core().id()
            }
        }

        impl StaticModuleCore for $ident {
            fn module_core(&self) -> &ModuleCore {
                &self.core
            }

            fn module_core_mut(&mut self) -> &mut ModuleCore {
                &mut self.core
            }
        }

        impl NameableModule for $ident {
            fn named(path: ModulePath, parameters: SpmcReader<Parameters>) -> Self {
                let mut this = Self::new();
                this.module_core_mut().parameters = parameters;
                this.module_core_mut().path = path;
                this
            }
        }

        impl NdlBuildableModule for $ident {}
    };
}

#[derive(Debug)]
struct Parent {
    core: ModuleCore,
    acummulated_counter: usize,
}

impl Parent {
    fn new() -> Self {
        Self {
            core: ModuleCore::new(),
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

impl Child {
    fn new() -> Self {
        Self {
            core: ModuleCore::new(),
            counter: 0,
        }
    }

    fn inc(&mut self, amount: usize) {
        self.counter += amount;
        self.parent_mut::<Parent>().unwrap().acummulated_counter += amount;
    }
}

auto_impl_static!(Child);

#[derive(Debug)]
struct GrandChild {
    core: ModuleCore,
}

impl GrandChild {
    fn new() -> Self {
        Self {
            core: ModuleCore::new(),
        }
    }
}

auto_impl_static!(GrandChild);

#[derive(Debug)]
struct TestCase {
    parent: Box<Parent>,
    children: Vec<Box<Child>>,
    grand_children: Vec<Box<GrandChild>>,
}

impl TestCase {
    fn build() -> Self {
        let mut parent = Box::new(Parent::named(
            ModulePath::root("Root".into()),
            SpmcWriter::new(Parameters::new()).get_reader(),
        ));

        let mut children = vec![
            Child::named_with_parent("c1", &mut *parent),
            Child::named_with_parent("c2", &mut *parent),
            Child::named_with_parent("c3", &mut *parent),
        ];

        let grand_children = vec![
            GrandChild::named_with_parent("left", &mut *children[0]),
            GrandChild::named_with_parent("right", &mut *children[0]),
            GrandChild::named_with_parent("left", &mut *children[1]),
            GrandChild::named_with_parent("right", &mut *children[1]),
            GrandChild::named_with_parent("left", &mut *children[2]),
            GrandChild::named_with_parent("right", &mut *children[2]),
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
    let _case = TestCase::build();
}

#[test]
fn test_parent_ptr() {
    let case = TestCase::build();

    println!("Parent: {:?}", TypeId::of::<Parent>());
    println!("Child: {:?}", TypeId::of::<Child>());
    println!("GrandChild: {:?}", TypeId::of::<GrandChild>());

    println!("{:?}", case.children[0]);

    assert_eq!(
        case.children[0].parent::<Parent>().unwrap().id(),
        case.parent.id()
    );
    assert_eq!(
        case.children[1].parent::<Parent>().unwrap().id(),
        case.parent.id()
    );
    assert_eq!(
        case.children[2].parent::<Parent>().unwrap().id(),
        case.parent.id()
    );

    assert_eq!(
        case.grand_children[0].parent::<Child>().unwrap().id(),
        case.children[0].id()
    );
    assert_eq!(
        case.grand_children[1].parent::<Child>().unwrap().id(),
        case.children[0].id()
    );
    assert_eq!(
        case.grand_children[2].parent::<Child>().unwrap().id(),
        case.children[1].id()
    );
}

#[test]
fn test_parent_mut_ptr() {
    let mut case = TestCase::build();

    case.children[0].inc(1);
    case.children[1].inc(2);
    case.children[2].inc(3);

    assert_eq!(case.children[0].counter, 1);
    assert_eq!(case.children[1].counter, 2);
    assert_eq!(case.children[2].counter, 3);

    assert_eq!(case.parent.acummulated_counter, 6);
}
