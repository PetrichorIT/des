use std::rc::Rc;

use crate::*;

///
/// A trait that prepares a module to be created from a NDL
/// file.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait NameableModule: StaticModuleCore {
    ///
    /// Creates a named instance of self without needing any additional parameters.
    ///
    fn named(path: ModulePath, parameters: Rc<Parameters>) -> Self;

    ///
    /// Creates a named instance of self based on the parent hierachical structure.
    ///
    #[allow(clippy::borrowed_box)]
    fn named_with_parent<T: NameableModule>(name: &str, parent: &Box<T>) -> Self
    where
        Self: Sized,
    {
        // Clippy is just confused .. non box-borrow would throw E0277

        Self::named(
            ModulePath::new_with_parent(name, parent.path()),
            parent.module_core().parameters.clone(),
        )
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
        let mut obj = Box::new(Self::named_with_parent(name, parent));
        obj.set_parent(parent);
        Self::build(obj, rt)
    }
}
