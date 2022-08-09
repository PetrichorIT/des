use crate::{create_global_uid, net::*, util::*};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

///
/// A type that contains a SubmoduleCore.
///
pub trait StaticSubsystemCore:
    Deref<Target = SubsystemCore> + DerefMut<Target = SubsystemCore>
{
}

impl<T> StaticSubsystemCore for T where
    T: Deref<Target = SubsystemCore> + DerefMut<Target = SubsystemCore>
{
}

create_global_uid!(
    /// A runtime-unqiue identifier for a module / submodule inheritence tree.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub SubsystemId(u16) = MODULE_ID;
);

///
/// The core primitive found on every submodule.
///
#[derive(Debug)]
pub struct SubsystemCore {
    pub(crate) id: SubsystemId,
    pub(crate) path: ObjectPath,

    pub(crate) channels: Vec<PtrMut<Channel>>,

    pub(crate) parent: Option<PtrWeakMut<dyn StaticSubsystemCore>>,
    pub(crate) children: HashMap<String, PtrWeakMut<dyn StaticSubsystemCore>>,

    pub(crate) globals: PtrWeakConst<NetworkRuntimeGlobals>,
}

impl SubsystemCore {
    /// The id of the submodule.
    pub fn id(&self) -> SubsystemId {
        self.id
    }

    ///
    /// A runtime-unqiue (not enforced) identifier for this module, based on its
    /// place in the module tree.
    ///
    pub fn path(&self) -> &ObjectPath {
        &self.path
    }

    ///
    /// Returns a human readable representation of the modules identity.
    ///
    pub fn str(&self) -> &str {
        self.path.path()
    }

    ///
    /// Returns the name of the module instance.
    ///
    pub fn name(&self) -> &str {
        self.path.name()
    }

    ///
    /// All channels managed by this subsystem.
    ///
    pub fn channels(&self) -> &[PtrMut<Channel>] {
        &self.channels
    }

    ///
    /// Creates a new optionally named instance
    /// of 'Self'.
    ///
    #[must_use]
    pub fn new_with(path: ObjectPath, globals: PtrWeakConst<NetworkRuntimeGlobals>) -> Self {
        Self {
            id: SubsystemId::gen(),
            path,
            parent: None,
            channels: Vec::new(),
            children: HashMap::new(),
            globals,
        }
    }

    ///
    /// Creates a new module core based on the parent
    /// using the name to extend the path.
    ///
    #[must_use]
    pub fn child_of(name: &str, parent: &SubsystemCore) -> Self {
        let path = ObjectPath::module_with_parent(name, &parent.path);

        Self {
            id: SubsystemId::gen(),
            path,
            parent: None,
            channels: Vec::new(),
            children: HashMap::new(),
            globals: parent.globals.clone(),
        }
    }
}

impl Default for SubsystemCore {
    fn default() -> Self {
        Self {
            id: SubsystemId::gen(),
            path: ObjectPath::root_subsystem("SIM".to_string()),
            channels: Vec::new(),
            parent: None,
            children: HashMap::new(),
            globals: PtrWeakConst::new(),
        }
    }
}
