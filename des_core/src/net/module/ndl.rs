use crate::{util::spmc::SpmcReader, *};

///
/// A trait that prepares a module to be created from a NDL
/// file.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait NameableModule: 'static + StaticModuleCore {
    ///
    /// Creates a named instance of self without needing any additional parameters.
    ///
    fn named(path: ModulePath, parameters: SpmcReader<Parameters>) -> Self;

    ///
    /// Creates a boxed instance of Self, based on the implementation of 'named'.
    ///
    fn named_boxed(path: ModulePath, parameters: SpmcReader<Parameters>) -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Self::named(path, parameters))
    }

    ///
    /// Creates a named instance of self based on the parent hierachical structure.
    ///
    fn named_with_parent<T>(name: &str, parent: &mut T) -> Box<Self>
    where
        T: NameableModule,
        Self: Sized,
    {
        let mut this = Self::named_boxed(
            ModulePath::new_with_parent(name, parent.path()),
            parent.module_core().parameters.clone(),
        );

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
pub trait NdlBuildableModule: StaticModuleCore {
    ///
    /// Builds the given module according to the NDL specification
    /// if any is provided, else doesn't change a thing.
    ///
    fn build<A>(self: Box<Self>, _rt: &mut NetworkRuntime<A>) -> Box<Self>
    where
        Self: Sized,
    {
        self
    }

    fn build_named<A>(path: ModulePath, rt: &mut NetworkRuntime<A>) -> Box<Self>
    where
        Self: NameableModule + Sized,
    {
        let obj = Box::new(Self::named(path, rt.parameters()));
        Self::build(obj, rt)
    }

    fn build_named_with_parent<A, T>(
        name: &str,
        parent: &mut Box<T>,
        rt: &mut NetworkRuntime<A>,
    ) -> Box<Self>
    where
        T: NameableModule,
        Self: NameableModule + Sized,
    {
        let mut obj = Self::named_with_parent(name, &mut **parent);
        parent.add_child(&mut (*obj));
        Self::build(obj, rt)
    }
}
