use std::marker::Unsize;

use crate::prelude::{Module, NetworkRuntime, ObjectPath};
use crate::{net::module::*, util::*};

use super::{Channel, StaticSubsystemCore};

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
        self.rt.include_par_file(file)
    }

    ///
    /// Returns the globals
    ///
    pub fn globals_weak(&self) -> PtrWeakConst<super::NetworkRuntimeGlobals> {
        self.rt.globals_weak()
    }

    ///
    /// Registers a module in the current runtime.
    ///
    pub fn create_module<T>(&mut self, module: PtrMut<T>)
    where
        T: Module + Unsize<dyn Module>,
    {
        self.rt.create_module(module)
    }

    /// Creates a channnel
    pub fn create_channel(&mut self, channel: PtrMut<Channel>) {
        if let Some(top) = self.sys_stack.last_mut() {
            (**top).channels.push(channel)
        }
    }

    /// Pushes a value
    pub fn push_subsystem<T>(&mut self, subsystem: PtrMut<T>)
    where
        T: StaticSubsystemCore + Unsize<dyn StaticSubsystemCore>,
    {
        let dyned: PtrMut<dyn StaticSubsystemCore> = subsystem;
        self.sys_stack.push(dyned)
    }

    /// Pops a value.
    pub fn pop_subsystem(&mut self) {
        self.sys_stack.pop();
    }
}

///
/// A trait that prepares a module to be created from a NDL
/// file.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait NameableModule: 'static + StaticModuleCore {
    ///
    /// Creates a named instance of the module with a provided [ModuleCore].
    ///
    /// Never call direct
    fn named(core: ModuleCore) -> Self;

    ///
    /// Creates a named instance at the root.
    ///
    fn named_root(core: ModuleCore) -> PtrMut<Self>
    where
        Self: Sized,
    {
        let mut this = PtrMut::new(Self::named(core));
        this.module_core_mut().self_ref = Some(PtrWeakVoid::new(PtrWeakMut::from_strong(&this)));

        this
    }

    ///
    /// Creates a named instance of self based on the parent hierachical structure.
    ///
    fn named_with_parent<T>(name: &str, parent: &mut PtrMut<T>) -> PtrMut<Self>
    where
        T: NameableModule,
        Self: Sized,
    {
        let core = ModuleCore::child_of(name, parent.module_core());
        let mut this = PtrMut::new(Self::named(core));
        this.module_core_mut().self_ref = Some(PtrWeakVoid::new(PtrWeakMut::from_strong(&this)));

        parent.add_child(&mut this);
        this
    }
}

macro_rules! impl_buildable {
    ($($g: ident),*) => {
        fn build<A$(,$g: Module + NameableModule + __Buildable0)*>(this: PtrMut<Self>, rt: &mut BuildContext<'_, A>) -> PtrMut<Self>
            where Self: Sized;

        fn build_named<A$(,$g: Module + NameableModule + __Buildable0)*>(path: ObjectPath, rt: &mut BuildContext<'_, A>) -> PtrMut<Self>
            where
                Self: NameableModule + Sized,
        {
            let core = ModuleCore::new_with(path, rt.globals_weak());
            let mut this = Ptr::new(Self::named(core));

            // Attach self to module core
            let clone = PtrWeak::from_strong(&this);
            this.deref_mut().self_ref = Some(PtrWeakVoid::new(clone));

            Self::build::<A$(,$g)*>(this, rt)
        }

        fn build_named_with_parent<A, T$(,$g: Module + NameableModule + __Buildable0)*>(
            name: &str,
            parent: &mut PtrMut<T>,
            rt: &mut BuildContext<'_, A>,
        ) -> PtrMut<Self>
            where
                T: NameableModule,
                Self: NameableModule + Sized,
        {
            let obj = Self::named_with_parent(name, parent);
            // parent.add_child(&mut (*obj));
            Self::build::<A$(,$g)*>(obj, rt)
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
    fn build<A>(this: PtrMut<Self>, _ctx: &mut BuildContext<'_, A>) -> PtrMut<Self>
    where
        Self: Sized,
    {
        this
    }

    fn build_named<A>(path: ObjectPath, ctx: &mut BuildContext<'_, A>) -> PtrMut<Self>
    where
        Self: NameableModule + Sized,
    {
        let core = ModuleCore::new_with(path, ctx.globals_weak());
        let mut this = PtrMut::new(Self::named(core));
        // Attach self to module core
        let clone = PtrWeak::from_strong(&this);
        this.deref_mut().self_ref = Some(PtrWeakVoid::new(clone));

        Self::build(this, ctx)
    }

    fn build_named_with_parent<A, T>(
        name: &str,
        parent: &mut PtrMut<T>,
        ctx: &mut BuildContext<'_, A>,
    ) -> PtrMut<Self>
    where
        T: NameableModule,
        Self: NameableModule + Sized,
    {
        let mut this = Self::named_with_parent(name, parent);

        let clone = PtrWeak::from_strong(&this);
        this.deref_mut().self_ref = Some(PtrWeakVoid::new(clone));

        Self::build(this, ctx)
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
