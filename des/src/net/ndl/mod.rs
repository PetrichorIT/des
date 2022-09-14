use std::cell::RefCell;
use std::marker::Unsize;
use std::sync::Arc;

use crate::prelude::{Module, NetworkRuntime};
use crate::util::PtrMut;

use super::module::ModuleContext;
use super::{ChannelRef, ModuleRef, ObjectPath, StaticSubsystemCore};

#[doc(hidden)]
#[derive(Debug)]
pub struct BuildContext<'a, A> {
    rt: &'a mut NetworkRuntime<A>,
    sys_stack: Vec<PtrMut<dyn StaticSubsystemCore>>,
}

impl<'a, A> BuildContext<'a, A> {
    ///
    /// Creates a new instance of self.
    ///
    #[must_use]
    pub fn new(rt: &'a mut NetworkRuntime<A>) -> Self {
        Self {
            rt,
            sys_stack: Vec::with_capacity(4),
        }
    }

    /// The rt
    pub fn rt(&mut self) -> &mut NetworkRuntime<A> {
        self.rt
    }

    /// Includes the par file
    pub fn include_par_file(&mut self, file: &str) {
        self.rt.include_par_file(file);
    }

    ///
    /// Registers a module in the current runtime.
    ///
    pub fn create_module(&mut self, module: ModuleRef) {
        self.rt.create_module(module);
    }

    /// Creates a channnel
    pub fn create_channel(&mut self, channel: ChannelRef) {
        if let Some(top) = self.sys_stack.last_mut() {
            top.channels.push(channel);
        }
    }

    /// Pushes a value
    pub fn push_subsystem<T>(&mut self, subsystem: PtrMut<T>)
    where
        T: StaticSubsystemCore + Unsize<dyn StaticSubsystemCore>,
    {
        let dyned: PtrMut<dyn StaticSubsystemCore> = subsystem;
        self.sys_stack.push(dyned);
    }

    /// Pops a value.
    pub fn pop_subsystem(&mut self) {
        self.sys_stack.pop();
    }
}

macro_rules! impl_buildable {
    ($($g: ident),*) => {
        fn build<A$(,$g: Module  + __Buildable0)*>(this: ModuleRef, rt: &mut BuildContext<'_, A>) -> ModuleRef
            where Self: Sized;

        fn build_named<A$(,$g: Module  + __Buildable0)*>(path: ObjectPath, rt: &mut BuildContext<'_, A>) -> ModuleRef
            where
                Self: 'static + Module + Sized,
        {
            let core = Arc::new(ModuleContext::standalone(path));
            let this = Arc::new(RefCell::new(<Self as Module>::new()));

            let mref = ModuleRef {
                ctx: core,
                handler: this,
            };

            mref.activiate();
            Self::build::<A$(,$g)*>(mref, rt)
        }

        fn build_named_with_parent<A, T$(,$g: Module  + __Buildable0)*>(
            name: &str,
            parent: ModuleRef,
            rt: &mut BuildContext<'_, A>,
        ) -> ModuleRef
            where
                T: Module,
                Self: 'static +  Module + Sized,
        {
            let core = Arc::new(ModuleContext::child_of(name, parent));
            let this = Arc::new(RefCell::new(<Self as Module>::new()));

            let mref = ModuleRef {
                ctx: core,
                handler: this,
            };

            mref.activiate();
            Self::build::<A$(,$g)*>(mref, rt)
        }
    };

}

/// Trait used by ndl internally.
#[doc(hidden)]
pub trait __Buildable0 {
    ///
    /// Builds the given module according to the NDL specification
    /// if any is provided, else doesn't change a thing.
    ///
    fn build<A>(this: ModuleRef, _ctx: &mut BuildContext<'_, A>) -> ModuleRef
    where
        Self: Sized,
    {
        this
    }

    fn build_named<A>(path: ObjectPath, ctx: &mut BuildContext<'_, A>) -> ModuleRef
    where
        Self: 'static + Module + Sized,
    {
        let core = Arc::new(ModuleContext::standalone(path));
        // TODO: Maybe activate the core here to provide funtionality witin user defined call of new

        let this = Arc::new(RefCell::new(<Self as Module>::new()));
        let mref = ModuleRef {
            ctx: core,
            handler: this,
        };

        mref.activiate();
        Self::build(mref, ctx)
    }

    fn build_named_with_parent<A>(
        name: &str,
        parent: ModuleRef,
        ctx: &mut BuildContext<'_, A>,
    ) -> ModuleRef
    where
        Self: 'static + Module + Sized,
    {
        let core = Arc::new(ModuleContext::child_of(name, parent));
        // TODO: Maybe activate the core here to provide funtionality witin user defined call of new

        let this = Arc::new(RefCell::new(<Self as Module>::new()));
        let mref = ModuleRef {
            ctx: core,
            handler: this,
        };

        mref.activiate();
        Self::build(mref, ctx)
    }
}

/// Trait used by ndl internally.
#[doc(hidden)]
pub trait __Buildable1 {
    impl_buildable! { T1 }
}
/// Trait used by ndl internally.
#[doc(hidden)]
pub trait __Buildable2 {
    impl_buildable! { T0, T1 }
}

/// Trait used by ndl internally.
#[doc(hidden)]
pub trait __Buildable3 {
    impl_buildable! { T0, T1, T2 }
}

/// Trait used by ndl internally.
#[doc(hidden)]
pub trait __Buildable4 {
    impl_buildable! { T0, T1, T2, T3 }
}

/// Trait used by ndl internally.
#[doc(hidden)]
pub trait __Buildable5 {
    impl_buildable! { T0, T1, T2, T3, T4 }
}

/// Trait used by ndl internally.
#[doc(hidden)]
pub trait __Buildable6 {
    impl_buildable! { T0, T1, T2, T3, T4, T5 }
}

/// Trait used by ndl internally.
#[doc(hidden)]
pub trait __Buildable7 {
    impl_buildable! { T0, T1, T2, T3, T4, T5, T6 }
}
