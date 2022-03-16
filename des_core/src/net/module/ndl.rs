use crate::{util::Mrc, *};

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
    fn named(core: ModuleCore) -> Self;

    ///
    /// Creates a named instance of self based on the parent hierachical structure.
    ///
    fn named_with_parent<T>(name: &str, parent: &mut T) -> Mrc<Self>
    where
        T: NameableModule,
        Self: Sized,
    {
        let core = ModuleCore::child_of(name, parent.module_core());
        let mut this = Mrc::new(Self::named(core));

        parent.add_child(&mut *this);
        this
    }
}

///
/// A macro-implemented trait that constructs a instance of Self using a NDl
/// description.
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait BuildableModule: StaticModuleCore {
    ///
    /// Builds the given module according to the NDL specification
    /// if any is provided, else doesn't change a thing.
    ///
    fn build<A>(this: Mrc<Self>, _rt: &mut NetworkRuntime<A>) -> Mrc<Self>
    where
        Self: Sized,
    {
        this
    }

    fn build_named<A>(path: ModulePath, rt: &mut NetworkRuntime<A>) -> Mrc<Self>
    where
        Self: NameableModule + Sized,
    {
        let core = ModuleCore::new_with(path, rt.parameters());
        let this = Mrc::new(Self::named(core));
        Self::build(this, rt)
    }

    fn build_named_with_parent<A, T>(
        name: &str,
        parent: &mut Mrc<T>,
        rt: &mut NetworkRuntime<A>,
    ) -> Mrc<Self>
    where
        T: NameableModule,
        Self: NameableModule + Sized,
    {
        let obj = Self::named_with_parent(name, &mut **parent);
        // parent.add_child(&mut (*obj));
        Self::build(obj, rt)
    }
}
