use super::{SubsystemId, SubsystemRef};
use crate::net::{ChannelRef, ObjectPath};
use std::{cell::RefCell, collections::HashMap};

///
/// The core primitive found on every submodule.
///
#[derive(Debug)]
pub struct SubsystemContext {
    pub(crate) id: SubsystemId,
    pub(crate) path: ObjectPath,

    pub(crate) channels: RefCell<Vec<ChannelRef>>,

    pub(crate) parent: Option<SubsystemRef>,
    pub(crate) children: HashMap<String, SubsystemRef>,
}

impl SubsystemContext {
    /// The id of the submodule.
    #[must_use]
    pub fn id(&self) -> SubsystemId {
        self.id
    }

    ///
    /// A runtime-unqiue (not enforced) identifier for this module, based on its
    /// place in the module tree.
    ///
    #[must_use]
    pub fn path(&self) -> &ObjectPath {
        &self.path
    }

    ///
    /// Returns a human readable representation of the modules identity.
    ///
    #[must_use]
    pub fn str(&self) -> &str {
        self.path.path()
    }

    ///
    /// Returns the name of the module instance.
    ///
    #[must_use]
    pub fn name(&self) -> &str {
        self.path.name()
    }

    ///
    /// All channels managed by this subsystem.
    ///
    #[must_use]
    pub fn channels(&self) -> Vec<ChannelRef> {
        self.channels.borrow().clone()
    }

    ///
    /// Creates a new optionally named instance
    /// of 'Self'.
    ///
    #[must_use]
    pub fn new_with(path: ObjectPath) -> Self {
        Self {
            id: SubsystemId::gen(),
            path,
            parent: None,
            channels: RefCell::new(Vec::new()),
            children: HashMap::new(),
        }
    }

    ///
    /// Creates a new module core based on the parent
    /// using the name to extend the path.
    ///
    #[must_use]
    pub fn child_of(name: &str, parent: &SubsystemRef) -> Self {
        let path = ObjectPath::module_with_parent(name, &parent.ctx.path);

        Self {
            id: SubsystemId::gen(),
            path,
            parent: None,
            channels: RefCell::new(Vec::new()),
            children: HashMap::new(),
        }
    }
}

impl Default for SubsystemContext {
    fn default() -> Self {
        Self {
            id: SubsystemId::gen(),
            path: ObjectPath::root_subsystem("SIM".to_string()),
            channels: RefCell::new(Vec::new()),
            parent: None,
            children: HashMap::new(),
        }
    }
}
