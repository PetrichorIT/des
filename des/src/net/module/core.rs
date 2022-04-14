use std::{
    any::type_name,
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
};

use log::error;

use crate::{
    core::SimTime,
    create_global_uid,
    net::{common::Optional, *},
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

    /// A offset for the out_buffer,
    pub(crate) processing_time_delay: SimTime,

    /// A buffer of messages to be send out, after the current handle messsage terminates.
    pub(crate) out_buffer: Vec<(Message, GateRef, SimTime)>,

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

    /// A refence to one self
    pub(crate) self_ref: Option<UntypedMrc>,
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
    /// Returns a ref unstructured list of all gates from the current module.
    ///
    pub fn gates(&self) -> &Vec<GateRefMut> {
        &self.gates
    }

    ///
    /// Returns a mutable ref to the all gates list.
    ///
    pub fn gates_mut(&mut self) -> &mut Vec<GateRefMut> {
        &mut self.gates
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    pub fn gate_cluster(&self, name: &str) -> Vec<&GateRefMut> {
        self.gates()
            .iter()
            .filter(|&gate| gate.name() == name)
            .collect()
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    pub fn gate_cluster_mut(&mut self, name: &str) -> Vec<&mut GateRefMut> {
        self.gates_mut()
            .iter_mut()
            .filter(|gate| gate.name() == name)
            .collect()
    }

    ///
    /// Returns a ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    pub fn gate(&self, name: &str, pos: usize) -> Option<GateRef> {
        Some(
            MrcS::clone(
                self.gates()
                    .iter()
                    .find(|&gate| gate.name() == name && gate.pos() == pos)?,
            )
            .make_readonly(),
        )
    }

    ///
    /// Returns a mutable ref to a gate of the current module dependent on its name and cluster position
    /// if possible.
    ///
    pub fn gate_mut(&mut self, name: &str, pos: usize) -> Option<&mut GateRefMut> {
        self.gates_mut()
            .iter_mut()
            .find(|gate| gate.name() == name && gate.pos() == pos)
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
            processing_time_delay: SimTime::ZERO,
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
            parent: None,
            children: HashMap::new(),
            globals,
            self_ref: None,
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
            processing_time_delay: SimTime::ZERO,
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
            parent: None,
            children: HashMap::new(),
            globals: parent.globals.clone(),
            self_ref: None,
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
    /// Adds the duration to the processing time offset.
    /// All messages send after this time will be delayed by the
    /// processing time delay.
    ///
    pub fn processing_time(&mut self, duration: SimTime) {
        self.processing_time_delay += duration;
    }

    ///
    /// Sends a message onto a given gate. This operation will be performed after
    /// handle_message finished.
    ///
    pub fn send<T>(&mut self, msg: impl Into<Message>, gate: T)
    where
        T: IntoModuleGate,
    {
        let gate = gate.into_gate(self);
        if let Some(gate) = gate {
            self.out_buffer
                .push((msg.into(), gate, self.processing_time_delay))
        } else {
            error!(target: self.str(),"Error: Could not find gate in current module");
        }
    }

    ///
    /// Enqueues a event that will trigger the [Module::handle_message] function
    /// in duration seconds, shifted by the processing time delay.
    ///
    pub fn schedule_in(&mut self, msg: impl Into<Message>, duration: SimTime) {
        assert!(
            duration >= SimTime::ZERO,
            "While we could maybe do this, we should not timetravel yet!"
        );
        self.loopback_buffer
            .push((msg.into(), self.processing_time_delay + duration))
    }

    ///
    /// Enqueues a event that will trigger the [Module::handle_message] function
    /// at the given SimTime
    ///
    pub fn schedule_at(&mut self, msg: impl Into<Message>, time: SimTime) {
        assert!(
            time >= SimTime::now() + self.processing_time_delay,
            "Sorry, you can not timetravel as well!"
        );
        self.loopback_buffer.push((msg.into(), time))
    }

    ///
    /// Enables the activity corountine using the given period.
    /// This function should only be called from [Module::handle_message].
    ///
    pub fn enable_activity(&mut self, period: SimTime) {
        self.activity_period = period;
        self.activity_active = false;
    }

    ///
    /// Disables the activity coroutine cancelling the next call.
    ///
    pub fn disable_activity(&mut self) {
        self.activity_period = SimTime::ZERO;
        self.activity_active = false;
    }
}

impl ModuleCore {
    ///
    /// Returns wether the moudule attached to this core is of type T.
    ///
    pub fn is<T>(&self) -> bool
    where
        T: 'static + StaticModuleCore,
    {
        self.self_ref
            .as_ref()
            .map(UntypedMrc::is::<T>)
            .unwrap_or(false)
    }

    ///
    /// Returns wether the moudule attached to this core is of type T.
    ///
    pub fn self_as<T>(&self) -> Option<MrcS<T, Mutable>>
    where
        T: 'static + Module,
    {
        let a = self.self_ref.as_ref().unwrap().clone();
        a.downcast()
    }

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
            None => Err(ModuleReferencingError::NoEntry(format!(
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
            None => Err(ModuleReferencingError::NoEntry(format!(
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
            None => Err(ModuleReferencingError::NoEntry(format!(
                "This module does not posses a child called '{}'",
                name
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
            None => Err(ModuleReferencingError::NoEntry(format!(
                "This module does not posses a child called '{}'",
                name
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
    /// Returns a parameter by reference (not parsed).
    ///
    pub fn par<'a>(&'a self, key: &'a str) -> ParHandle<'a, Optional> {
        self.globals.parameters.get_handle(self.path.path(), key)
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
    NoEntry(String),
    TypeError(String),
}

impl Display for ModuleReferencingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEntry(str) => write!(f, "{}", str),
            Self::TypeError(str) => write!(f, "{}", str),
        }
    }
}

impl Error for ModuleReferencingError {}
