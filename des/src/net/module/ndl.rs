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
