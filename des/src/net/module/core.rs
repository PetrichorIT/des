use std::{
    any::type_name,
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
};

use crate::{
    core::SimTime,
    create_global_uid,
    net::*,
    util::{MrcS, Mutable, ReadOnly, UntypedMrc},
};

create_global_uid!(
    /// A runtime-unqiue identifier for a module / submodule inheritence tree.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub ModuleId(u16) = MODULE_ID;
);

///
/// The usecase independent core of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Clone)]
pub struct ModuleCore {
    id: ModuleId,

    /// A human readable identifier for the module.
    pub(crate) path: ModulePath,

    /// A collection of all gates register to the current module
    pub(crate) gates: Vec<GateRefMut>,

    /// A buffer of messages to be send out, after the current handle messsage terminates.
    pub(crate) out_buffer: Vec<(Message, GateRef)>,

    /// A buffer of wakeup calls to be enqueued, after the current handle message terminates.
    pub(crate) loopback_buffer: Vec<(Message, SimTime)>,

    /// The period of the activity coroutine (if zero than there is no coroutine).
    pub(crate) activity_period: SimTime,

    /// An indicator whether a valid activity timeout is existent.
    pub(crate) activity_active: bool,

    /// The reference for the parent module.
    pub(crate) parent: Option<UntypedMrc>,

    /// The collection of child nodes for the current module.
    pub(crate) children: HashMap<String, UntypedMrc>,

    /// A set of local parameters.
    globals: MrcS<NetworkRuntimeGlobals, ReadOnly>,
}

impl ModuleCore {
    ///
    /// A runtime-unqiue identifier for this module-core and by extension this module.
    ///
    pub fn id(&self) -> ModuleId {
        self.id
    }

    ///
    /// A runtime-unqiue (not enforced) identifier for this module, based on its
    /// place in the module tree.
    ///
    pub fn path(&self) -> &ModulePath {
        &self.path
    }

    ///
    /// Creates a new optionally named instance
    /// of 'Self'.
    ///
    pub fn new_with(path: ModulePath, globals: MrcS<NetworkRuntimeGlobals, ReadOnly>) -> Self {
        Self {
            id: ModuleId::gen(),
            path,
            gates: Vec::new(),
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
            parent: None,
            children: HashMap::new(),
            globals,
        }
    }

    ///
    /// Creates a new module core based on the parent
    /// using the name to extend the path.
    ///
    pub fn child_of(name: &str, parent: &ModuleCore) -> Self {
        let path = ModulePath::new_with_parent(name, &parent.path);

        Self {
            id: ModuleId::gen(),
            path,
            gates: Vec::new(),
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
            parent: None,
            children: HashMap::new(),
            globals: parent.globals.clone(),
        }
    }

    ///
    /// Creates  a not-named instance of 'Self'.
    ///
    #[inline(always)]
    pub fn new() -> Self {
        Self::new_with(
            ModulePath::root(String::from("unknown-module")),
            MrcS::new(NetworkRuntimeGlobals::new()),
        )
    }
}

impl ModuleCore {
    ///
    /// Returns the parent module by reference if a parent exists
    /// and is of type `T`.
    ///
    pub fn parent<T>(&self) -> Result<MrcS<T, ReadOnly>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        match self.parent.clone() {
            Some(parent) => match parent.downcast::<T>() {
                Some(parent) => Ok(parent.make_readonly()),
                None => Err(ModuleReferencingError::TypeError(format!(
                    "The parent module of '{}' is not of type {}",
                    self.path(),
                    type_name::<T>(),
                ))),
            },
            None => Err(ModuleReferencingError::NoParent(format!(
                "The module '{}' does not posses a parent ptr",
                self.path()
            ))),
        }
    }

    ///
    /// Returns the parent module by mutable reference if a parent exists
    /// and is of type `T`.
    ///
    pub fn parent_mut<T>(&mut self) -> Result<MrcS<T, Mutable>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        match self.parent.clone() {
            Some(parent) => match parent.downcast::<T>() {
                Some(parent) => Ok(parent),
                None => Err(ModuleReferencingError::TypeError(format!(
                    "The parent module of '{}' is not of type {}",
                    self.path(),
                    type_name::<T>(),
                ))),
            },
            None => Err(ModuleReferencingError::NoParent(format!(
                "The module '{}' does not posses a parent ptr",
                self.path()
            ))),
        }
    }

    ///
    /// Returns the child module by reference if any child with
    /// the given name exists and is of type `T`.
    ///
    pub fn child<T>(&self, name: &str) -> Result<MrcS<T, ReadOnly>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        match self.children.get(name) {
            Some(parent) => {
                // Text
                match parent.clone().downcast::<T>() {
                    Some(parent) => Ok(parent.make_readonly()),
                    None => Err(ModuleReferencingError::TypeError(String::from(
                        "Type error",
                    ))),
                }
            }
            None => Err(ModuleReferencingError::NoParent(String::from(
                "This module does not posses a parent ptr",
            ))),
        }
    }

    ///
    /// Returns the child module by mutable reference if any child with
    /// the given name exists and is of type `T`.
    ///
    pub fn child_mut<T>(&mut self, name: &str) -> Result<MrcS<T, Mutable>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
        Self: 'static + Sized,
    {
        match self.children.get(name) {
            Some(parent) => {
                // Text
                match parent.clone().downcast::<T>() {
                    Some(parent) => Ok(parent),
                    None => Err(ModuleReferencingError::TypeError(String::from(
                        "Type error",
                    ))),
                }
            }
            None => Err(ModuleReferencingError::NoParent(String::from(
                "This module does not posses a parent ptr",
            ))),
        }
    }
}

///
/// # Parameter management
///

impl ModuleCore {
    ///
    /// Returns the parameters for the current module.
    ///
    pub fn pars(&self) -> HashMap<String, String> {
        self.globals.parameters.get(self.path.path())
    }

    ///
    /// Returns a reference to the parameter store, used for constructing
    /// custom instances of modules.
    ///
    pub fn globals(&self) -> MrcS<NetworkRuntimeGlobals, ReadOnly> {
        MrcS::clone(&self.globals)
    }
}

impl Default for ModuleCore {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ModuleCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: more exhaustive debug struct
        f.debug_struct("ModuleCore")
            .field("id", &self.id)
            .field("path", &self.path)
            .field("gates", &self.gates)
            .finish()
    }
}

///
/// An error while resolving a reference to another module.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleReferencingError {
    NoParent(String),
    TypeError(String),
}

impl Display for ModuleReferencingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoParent(str) => write!(f, "{}", str),
            Self::TypeError(str) => write!(f, "{}", str),
        }
    }
}

impl Error for ModuleReferencingError {}
