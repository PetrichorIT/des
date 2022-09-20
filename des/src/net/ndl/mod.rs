use std::sync::Arc;

use crate::prelude::{Module, NetworkRuntime};

use super::module::ModuleContext;
use super::subsystem::SubsystemRef;
use super::{channel::ChannelRef, module::ModuleRef, ObjectPath};

#[doc(hidden)]
#[derive(Debug)]
pub struct BuildContext<'a, A> {
    rt: &'a mut NetworkRuntime<A>,
    sys_stack: Vec<SubsystemRef>,
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
            top.ctx.channels.borrow_mut().push(channel);
        }
    }

    /// Pushes a value
    pub fn push_subsystem(&mut self, subsystem: SubsystemRef) {
        self.sys_stack.push(subsystem);
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
            ModuleContext::place(Arc::clone(&core));

            let this = <Self as Module>::new();
            let mref = ModuleRef::new(core, this);

            // mref.activate();
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
            ModuleContext::place(Arc::clone(&core));

            let this = <Self as Module>::new();
            let mref = ModuleRef::new(core, this);

            // mref.activate();
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
        ModuleContext::place(Arc::clone(&core));
        // TODO: Maybe activate the core here to provide funtionality witin user defined call of new

        let this = <Self as Module>::new();
        let mref = ModuleRef::new(core, this);

        // mref.activate();
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
        ModuleContext::place(Arc::clone(&core));
        // TODO: Maybe activate the core here to provide funtionality witin user defined call of new

        let this = <Self as Module>::new();
        let mref = ModuleRef::new(core, this);

        // mref.activate();
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
