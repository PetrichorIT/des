use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
};

use log::error;

use crate::{
    create_global_uid,
    net::{
        common::Optional, GateRef, GateRefMut, IntoModuleGate, Message, NetworkRuntimeGlobals,
        ObjectPath, ParHandle, StaticModuleCore,
    },
    time::{Duration, SimTime},
    util::{Ptr, PtrConst, PtrWeakConst, PtrWeakMut, PtrWeakVoid},
};

create_global_uid!(
    /// A runtime-unqiue identifier for a module / submodule inheritence tree.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub ModuleId(u16) = MODULE_ID;
);

///
/// A managment struct for the senders.
///
#[derive(Debug)]
pub(crate) struct ModuleBuffer {
    pub(crate) out_buffer: Vec<(Message, GateRef, SimTime)>,
    pub(crate) loopback_buffer: Vec<(Message, SimTime)>,

    pub(crate) processing_time_delay: Duration,
}

impl ModuleBuffer {
    ///
    /// Adds the given duration to the processing time.
    /// Note that all buffer handles have their own processing time.
    ///
    pub(crate) fn processing_time(&mut self, duration: Duration) {
        self.processing_time_delay += duration;
    }

    pub(crate) fn new() -> Self {
        Self {
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            processing_time_delay: Duration::ZERO,
        }
    }

    pub(crate) fn send(&mut self, msg: impl Into<Message>, gate: GateRef) {
        self.out_buffer.push((msg.into(), gate, SimTime::now()));
    }

    pub(crate) fn send_in(&mut self, msg: impl Into<Message>, gate: GateRef, delay: Duration) {
        self.out_buffer
            .push((msg.into(), gate, SimTime::now() + delay));
    }

    pub(crate) fn schedule_in(&mut self, msg: impl Into<Message>, duration: Duration) {
        assert!(
            duration >= Duration::ZERO,
            "While we could maybe do this, we should not timetravel yet!"
        );
        self.loopback_buffer
            .push((msg.into(), SimTime::now() + duration));
    }

    pub(crate) fn schedule_at(&mut self, msg: impl Into<Message>, time: SimTime) {
        assert!(
            time >= SimTime::now(),
            "Sorry, you can not timetravel as well!"
        );
        self.loopback_buffer.push((msg.into(), time));
    }
}

// SAFTEY:
// Send can be implemented since Send is only used when using AsyncModule
// which takes qusai-ownership of self. Additionally only the current thread is used
// so thread-specific datastructures will be preserved
// => This impl is redundant since ModuleCoreInner is Send+Sync but this is derived
//    by arguments presented here.
unsafe impl Send for ModuleCore {}
unsafe impl Sync for ModuleCore {}

///
/// The core values provided on any module.
///
pub struct ModuleCore {
    id: ModuleId,

    /// A human readable identifier for the module.
    pub(crate) path: ObjectPath,

    /// A collection of all gates register to the current module
    pub(crate) gates: Vec<GateRefMut>,

    /// The buffers for processing messages
    pub(crate) buffers: ModuleBuffer,

    /// Expensions for async
    #[cfg(feature = "async")]
    pub(crate) async_ext: super::ext::AsyncCoreExt,

    /// The period of the activity coroutine (if zero than there is no coroutine).
    pub(crate) activity_period: Duration,

    /// An indicator whether a valid activity timeout is existent.
    pub(crate) activity_active: bool,

    /// The reference for the parent module.
    pub(crate) parent: Option<PtrWeakMut<dyn StaticModuleCore>>,

    /// The collection of child nodes for the current module.
    pub(crate) children: HashMap<String, PtrWeakMut<dyn StaticModuleCore>>,

    /// A set of local parameters.
    globals: PtrWeakConst<NetworkRuntimeGlobals>,

    /// A refence to one self
    pub(crate) self_ref: Option<PtrWeakVoid>,
}

impl ModuleCore {
    ///
    /// A runtime-unqiue identifier for this module-core and by extension this module.
    ///
    #[must_use]
    pub fn id(&self) -> ModuleId {
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
    /// Returns a ref unstructured list of all gates from the current module.
    ///
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn gate(&self, name: &str, pos: usize) -> Option<GateRef> {
        Some(
            Ptr::clone(
                self.gates()
                    .iter()
                    .find(|&gate| gate.name() == name && gate.pos() == pos)?,
            )
            .make_const(),
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
    #[must_use]
    pub fn new_with(path: ObjectPath, globals: PtrWeakConst<NetworkRuntimeGlobals>) -> Self {
        #[cfg(feature = "async")]
        let tctx_ident = path.path().to_string();

        Self {
            #[cfg(feature = "async")]
            async_ext: super::ext::AsyncCoreExt::new(tctx_ident),

            id: ModuleId::gen(),
            path,
            gates: Vec::new(),
            buffers: ModuleBuffer::new(),
            activity_period: Duration::ZERO,
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
    #[must_use]
    pub fn child_of(name: &str, parent: &ModuleCore) -> Self {
        let path = ObjectPath::module_with_parent(name, &parent.path);

        #[cfg(feature = "async")]
        let tctx_ident = path.path().to_string();

        Self {
            id: ModuleId::gen(),
            path,
            gates: Vec::new(),
            buffers: ModuleBuffer::new(),
            activity_period: Duration::ZERO,
            activity_active: false,
            parent: None,
            children: HashMap::new(),
            globals: parent.globals(),
            self_ref: None,

            #[cfg(feature = "async")]
            async_ext: super::ext::AsyncCoreExt::new(tctx_ident),
        }
    }

    ///
    /// Creates  a not-named instance of 'Self'.
    //7
    #[must_use]
    pub fn new() -> Self {
        Self::new_with(
            ObjectPath::root_module(String::from("unknown-module")),
            PtrWeakConst::from_strong(&PtrConst::new(NetworkRuntimeGlobals::new())),
        )
    }
}

impl ModuleCore {
    ///
    /// Returns a sendable handle for sending message.
    ///
    #[cfg(feature = "async")]
    #[must_use]
    pub fn async_handle(&self) -> super::SenderHandle {
        let inner = self.async_ext.handle.clone();

        super::SenderHandle {
            inner,
            time_offset: self.buffers.processing_time_delay,
        }
    }

    ///
    /// Adds the duration to the processing time offset.
    /// All messages send after this time will be delayed by the
    /// processing time delay.
    ///
    pub fn processing_time(&mut self, duration: Duration) {
        self.buffers.processing_time(duration);
    }

    ///
    /// Sends a message onto a given gate. This operation will be performed after
    /// `handle_message` finished.
    ///
    #[allow(clippy::needless_pass_by_value)]
    pub fn send(&mut self, msg: impl Into<Message>, gate: impl IntoModuleGate) {
        let gate = gate.as_gate(self);
        if let Some(gate) = gate {
            self.buffers.send(msg, gate);
        } else {
            error!(target: self.str(),"Error: Could not find gate in current module");
        }
    }

    ///
    /// Sends a message onto a given gate with a delay. This operation will be performed after
    /// `handle_message` finished.
    ///
    #[allow(clippy::needless_pass_by_value)]
    pub fn send_in(&mut self, msg: impl Into<Message>, gate: impl IntoModuleGate, delay: Duration) {
        let gate = gate.as_gate(self);
        if let Some(gate) = gate {
            self.buffers.send_in(msg, gate, delay);
        } else {
            error!(target: self.str(),"Error: Could not find gate in current module");
        }
    }

    ///
    /// Enqueues a event that will trigger the [`Module::handle_message`] function
    /// in duration seconds, shifted by the processing time delay.
    ///
    pub fn schedule_in(&mut self, msg: impl Into<Message>, duration: Duration) {
        self.buffers.schedule_in(msg, duration);
    }

    ///
    /// Enqueues a event that will trigger the [`Module::handle_message`] function
    /// at the given `SimTime`
    ///
    pub fn schedule_at(&mut self, msg: impl Into<Message>, time: SimTime) {
        self.buffers.schedule_at(msg, time);
    }

    ///
    /// Enables the activity corountine using the given period.
    /// This function should only be called from [`Module::handle_message`].
    ///
    pub fn enable_activity(&mut self, period: Duration) {
        self.activity_period = period;
        self.activity_active = false;
    }

    ///
    /// Disables the activity coroutine cancelling the next call.
    ///
    pub fn disable_activity(&mut self) {
        self.activity_period = Duration::ZERO;
        self.activity_active = false;
    }

    ///
    /// Shuts down all activity for the module.
    /// Restarts the module at the given time.
    ///
    #[cfg(feature = "async")]
    #[cfg(not(feature = "async-sharedrt"))]
    pub fn shutdown(&mut self, restart_at: Option<SimTime>) {
        assert!(restart_at.unwrap_or(SimTime::MAX) > SimTime::now());
        self.async_ext.rt.take();
        if let Some(restart_at) = restart_at {
            self.schedule_at(
                Message::new().typ(crate::net::message::TYP_RESTART).build(),
                restart_at,
            )
        }
    }
}

impl ModuleCore {
    ///
    /// Returns wether the moudule attached to this core is of type T.
    ///
    #[must_use]
    pub fn is<T>(&self) -> bool
    where
        T: 'static + StaticModuleCore,
    {
        self.self_ref.as_ref().map_or(false, PtrWeakVoid::is::<T>)
    }

    ///
    /// Returns wether the moudule attached to this core is of type T.
    ///
    /// # Panics
    ///
    /// Panics if no self-ref was set up.
    ///
    #[must_use]
    pub fn self_as<T>(&self) -> Option<PtrWeakMut<T>>
    where
        T: 'static + StaticModuleCore,
    {
        match self.self_ref.as_ref() {
            Some(r) => r.clone().downcast(),
            None => panic!("Missing self ref at {}", self.str()),
        }
        // let a = self.self_ref.as_ref().unwrap().clone();
        // a.downcast()
    }

    ///
    /// Returns the parent as a [`PtrWeakConst`].
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleReferencingError`].
    ///
    pub fn parent(&self) -> Result<PtrWeakConst<dyn StaticModuleCore>, ModuleReferencingError> {
        match self.parent.as_ref() {
            Some(parent) => Ok(PtrWeakMut::clone(parent).make_const()),
            None => Err(ModuleReferencingError::NoEntry(format!(
                "The module '{}' does not posses a parent ptr",
                self.path()
            ))),
        }
    }

    ///
    /// Returns the parent as a [`PtrWeakConst`] casted to the given type T.
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleReferencingError`].
    ///
    /// # Panics
    ///
    /// Panics if the module exists and is not of type T.
    ///
    pub fn parent_as<T>(&self) -> Result<PtrWeakConst<T>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        match self.parent.as_ref() {
            Some(parent) => Ok(parent.self_as::<T>().unwrap().make_const()),
            None => Err(ModuleReferencingError::NoEntry(format!(
                "The module '{}' does not posses a parent ptr",
                self.path()
            ))),
        }
    }

    ///
    /// Returns the parent as a [`PtrWeakMut`].
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleReferencingError`].
    ///
    pub fn parent_mut(
        &mut self,
    ) -> Result<PtrWeakMut<dyn StaticModuleCore>, ModuleReferencingError> {
        match self.parent.as_ref() {
            Some(parent) => Ok(PtrWeakMut::clone(parent)),
            None => Err(ModuleReferencingError::NoEntry(format!(
                "The module '{}' does not posses a parent ptr",
                self.path()
            ))),
        }
    }

    ///
    /// Returns the parent as a [`PtrWeakMut`] casted to the type T.
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleReferencingError`].
    ///
    /// # Panics
    ///
    /// Panics if the module exists and is not of type T.
    ///
    pub fn parent_mut_as<T>(&mut self) -> Result<PtrWeakMut<T>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        match self.parent.as_ref() {
            Some(parent) => Ok(parent.self_as::<T>().unwrap()),
            None => Err(ModuleReferencingError::NoEntry(format!(
                "The module '{}' does not posses a parent ptr",
                self.path()
            ))),
        }
    }

    ///
    /// Returns an iterator over all children idents.
    ///
    pub fn children(&self) -> impl Iterator<Item = &String> {
        self.children.keys()
    }

    ///
    /// Returns the child with the given name if existent, as a [`PtrWeakConst`].
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleReferencingError`].
    ///
    pub fn child(
        &self,
        name: &str,
    ) -> Result<PtrWeakConst<dyn StaticModuleCore>, ModuleReferencingError> {
        match self.children.get(name) {
            Some(child) => Ok(PtrWeakMut::clone(child).make_const()),
            None => Err(ModuleReferencingError::NoEntry(format!(
                "This module does not posses a child called '{}'",
                name
            ))),
        }
    }

    ///
    /// Returns the child with the given name if existent, as a [`PtrWeakConst`]
    /// casted to the type T.
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleReferencingError`].
    ///
    /// # Panics
    ///
    /// Panics if the module exists and is not of type T.
    ///
    pub fn child_as<T>(&self, name: &str) -> Result<PtrWeakConst<T>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        match self.children.get(name) {
            Some(child) => Ok(child.self_as::<T>().unwrap().make_const()),
            None => Err(ModuleReferencingError::NoEntry(format!(
                "This module does not posses a child called '{}'",
                name
            ))),
        }
    }

    ///
    /// Returns the child with the given name if existent, as a [`PtrWeakMut`].
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleReferencingError`].
    ///
    pub fn child_mut(
        &mut self,
        name: &str,
    ) -> Result<PtrWeakMut<dyn StaticModuleCore>, ModuleReferencingError> {
        match self.children.get(name) {
            Some(child) => Ok(PtrWeakMut::clone(child)),
            None => Err(ModuleReferencingError::NoEntry(format!(
                "This module does not posses a child called '{}'",
                name
            ))),
        }
    }

    ///
    /// Returns the child with the given name if existent, as a [`PtrWeakMut`]
    /// casted to the type T.
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleReferencingError`].
    ///
    /// # Panics
    ///
    /// Panics if the module exists and is not of type T.
    ///
    pub fn child_mut_as<T>(&mut self, name: &str) -> Result<PtrWeakMut<T>, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        match self.children.get(name) {
            Some(child) => Ok(child.self_as::<T>().unwrap()),
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
    #[must_use]
    pub fn pars(&self) -> HashMap<String, String> {
        self.globals.parameters.get_def_table(self.path.path())
    }

    ///
    /// Returns a parameter by reference (not parsed).
    ///
    #[must_use]
    pub fn par<'a>(&'a self, key: &'a str) -> ParHandle<'a, Optional> {
        self.globals.parameters.get_handle(self.path.path(), key)
    }

    ///
    /// Returns a reference to the parameter store, used for constructing
    /// custom instances of modules.
    ///
    #[must_use]
    pub fn globals(&self) -> PtrWeakConst<NetworkRuntimeGlobals> {
        PtrWeakConst::clone(&self.globals)
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
    /// No reference exists.
    NoEntry(String),
    /// The reference is not of the given type.
    TypeError(String),
}

impl Display for ModuleReferencingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEntry(str) | Self::TypeError(str) => write!(f, "{}", str),
        }
    }
}

impl Error for ModuleReferencingError {}
