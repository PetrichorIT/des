use std::fmt::{Debug, Display};

use super::*;

/// A runtime-unquie identifier for a gate.
pub type GateId = u32;
/// A non-initalized gate.
pub const GATE_NULL: GateId = 0;
/// A referecne to the current working gate.
pub const GATE_SELF: GateId = 1;

///
/// The type of a gate, defining the message flow
/// and the derived methods.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateType {
    /// A null type to identifie a undefined gate.
    None,
    /// A type that allows the receiving of messages.
    Input,
    /// A type that allows the sending of messages.
    Output,
    /// A gate in duplex mode.
    InOut,
}

impl Display for GateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

///
/// A description of a gate / gate cluster on a module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GateDescription {
    pub owner: ModuleId,
    pub name: String,
    pub size: usize,
    pub typ: GateType,
}

impl GateDescription {
    pub fn is_vector(&self) -> bool {
        self.size != 1
    }

    pub fn new(typ: GateType, name: String, size: usize, owner: ModuleId) -> Self {
        Self {
            typ,
            name,
            size,
            owner,
        }
    }
}

impl Display for GateDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

static mut NEXT_GATE_ID: GateId = 0xff;
fn get_gate_id() -> GateId {
    unsafe {
        let id = NEXT_GATE_ID;
        NEXT_GATE_ID += 1;
        id
    }
}

///
/// A gate, a message insertion or extraction point used for handeling channels.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Gate {
    id: GateId,

    description: GateDescription,
    pos: usize,

    channel_id: ChannelId,
    next_gate: GateId,
}

impl Gate {
    #[inline(always)]
    pub fn id(&self) -> GateId {
        self.id
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn name(&self) -> &String {
        &self.description.name
    }

    pub fn next_gate(&self) -> GateId {
        self.next_gate
    }

    pub fn set_next_gate(&mut self, next_gate: GateId) {
        self.next_gate = next_gate;
    }

    pub fn channel(&self) -> ChannelId {
        self.channel_id
    }

    pub fn module(&self) -> ModuleId {
        self.description.owner
    }

    pub fn new(
        description: GateDescription,
        pos: usize,
        channel: &Channel,
        next_gate: GateId,
    ) -> Self {
        Self {
            id: get_gate_id(),
            description,
            pos,
            channel_id: channel.id,
            next_gate,
        }
    }
}

// == NEW ==

pub trait IntoModuleGate<T: Module>: Sized {
    fn into_gate(self, _module: &T) -> Option<GateId> {
        None
    }
}

impl<T: Module> IntoModuleGate<T> for Gate {
    fn into_gate(self, module: &T) -> Option<GateId> {
        let element = module.gates().iter().find(|&g| g == &self)?;
        Some(element.id())
    }
}

impl<T: Module> IntoModuleGate<T> for GateId {
    fn into_gate(self, module: &T) -> Option<GateId> {
        let element = module.gates().iter().find(|&g| g.id() == self)?;
        Some(element.id())
    }
}

impl<T: Module> IntoModuleGate<T> for (&str, usize) {
    fn into_gate(self, module: &T) -> Option<GateId> {
        let element = module
            .gates()
            .iter()
            .find(|&g| g.name() == self.0 && g.pos() == self.1)?;

        Some(element.id())
    }
}
