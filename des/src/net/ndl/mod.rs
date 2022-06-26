use crate::prelude::{Module, NetworkRuntime, ObjectPath};
use crate::{net::module::*, util::*};

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
        fn build<A$(,$g: Module + NameableModule + __Buildable0)*>(this: PtrMut<Self>, rt: &mut NetworkRuntime<A>) -> PtrMut<Self>
            where Self: Sized;

        fn build_named<A$(,$g: Module + NameableModule + __Buildable0)*>(path: ObjectPath, rt: &mut NetworkRuntime<A>) -> PtrMut<Self>
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
            rt: &mut NetworkRuntime<A>,
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
    fn build<A>(this: PtrMut<Self>, _rt: &mut NetworkRuntime<A>) -> PtrMut<Self>
    where
        Self: Sized,
    {
        this
    }

    fn build_named<A>(path: ObjectPath, rt: &mut NetworkRuntime<A>) -> PtrMut<Self>
    where
        Self: NameableModule + Sized,
    {
        let core = ModuleCore::new_with(path, rt.globals_weak());
        let mut this = PtrMut::new(Self::named(core));
        // Attach self to module core
        let clone = PtrWeak::from_strong(&this);
        this.deref_mut().self_ref = Some(PtrWeakVoid::new(clone));

        Self::build(this, rt)
    }

    fn build_named_with_parent<A, T>(
        name: &str,
        parent: &mut PtrMut<T>,
        rt: &mut NetworkRuntime<A>,
    ) -> PtrMut<Self>
    where
        T: NameableModule,
        Self: NameableModule + Sized,
    {
        let mut this = Self::named_with_parent(name, parent);

        let clone = PtrWeak::from_strong(&this);
        this.deref_mut().self_ref = Some(PtrWeakVoid::new(clone));

        Self::build(this, rt)
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
