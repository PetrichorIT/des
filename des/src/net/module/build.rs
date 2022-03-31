use crate::{
    prelude::{Module, ModulePath, NetworkRuntime},
    util::{MrcS, Mutable, UntypedMrc},
};

use super::{ModuleCore, NameableModule};

macro_rules! impl_buildable {
    ($($g: ident),*) => {
        fn build<A$(,$g: Module + NameableModule + __Buildable0)*>(this: MrcS<Self, Mutable>, rt: &mut NetworkRuntime<A>) -> MrcS<Self, Mutable>
            where Self: Sized;

        fn build_named<A$(,$g: Module + NameableModule + __Buildable0)*>(path: ModulePath, rt: &mut NetworkRuntime<A>) -> MrcS<Self, Mutable>
            where
                Self: NameableModule + Sized,
        {
            let core = ModuleCore::new_with(path, rt.globals());
            let mut this = MrcS::new(Self::named(core));

            // Attach self to module core
            let clone = MrcS::clone(&this);
            this.deref_mut().self_ref = Some(UntypedMrc::new(clone));

            Self::build::<A$(,$g)*>(this, rt)
        }

        fn build_named_with_parent<A, T$(,$g: Module + NameableModule + __Buildable0)*>(
            name: &str,
            parent: &mut MrcS<T, Mutable>,
            rt: &mut NetworkRuntime<A>,
        ) -> MrcS<Self, Mutable>
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

/// Trait used by [ndl] internally.
pub trait __Buildable0 {
    ///
    /// Builds the given module according to the NDL specification
    /// if any is provided, else doesn't change a thing.
    ///
    fn build<A>(this: MrcS<Self, Mutable>, _rt: &mut NetworkRuntime<A>) -> MrcS<Self, Mutable>
    where
        Self: Sized,
    {
        this
    }

    fn build_named<A>(path: ModulePath, rt: &mut NetworkRuntime<A>) -> MrcS<Self, Mutable>
    where
        Self: NameableModule + Sized,
    {
        let core = ModuleCore::new_with(path, rt.globals());
        let mut this = MrcS::new(Self::named(core));

        // Attach self to module core
        let clone = MrcS::clone(&this);
        this.deref_mut().self_ref = Some(UntypedMrc::new(clone));

        Self::build(this, rt)
    }

    fn build_named_with_parent<A, T>(
        name: &str,
        parent: &mut MrcS<T, Mutable>,
        rt: &mut NetworkRuntime<A>,
    ) -> MrcS<Self, Mutable>
    where
        T: NameableModule,
        Self: NameableModule + Sized,
    {
        let obj = Self::named_with_parent(name, parent);
        // parent.add_child(&mut (*obj));
        Self::build(obj, rt)
    }
}

/// Trait used by [ndl] internally.
pub trait __Buildable1 {
    impl_buildable! { T1 }
}

/// Trait used by [ndl] internally.
pub trait __Buildable2 {
    impl_buildable! { T0, T1 }
}

/// Trait used by [ndl] internally.
pub trait __Buildable3 {
    impl_buildable! { T0, T1, T2 }
}

/// Trait used by [ndl] internally.
pub trait __Buildable4 {
    impl_buildable! { T0, T1, T2, T3 }
}

/// Trait used by [ndl] internally.
pub trait __Buildable5 {
    impl_buildable! { T0, T1, T2, T3, T4 }
}

/// Trait used by [ndl] internally.
pub trait __Buildable6 {
    impl_buildable! { T0, T1, T2, T3, T4, T5 }
}

/// Trait used by [ndl] internally.
pub trait __Buildable7 {
    impl_buildable! { T0, T1, T2, T3, T4, T5, T6 }
}
