use crate::{net::module::*, util::MrcS};

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
    fn named_with_parent<T>(name: &str, parent: &mut MrcS<T, Mutable>) -> MrcS<Self, Mutable>
    where
        T: NameableModule,
        Self: Sized,
    {
        let core = ModuleCore::child_of(name, parent.module_core());
        let mut this = MrcS::new(Self::named(core));

        parent.add_child(&mut this);
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
