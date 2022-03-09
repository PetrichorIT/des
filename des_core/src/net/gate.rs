use crate::create_global_uid;
use crate::net::*;
use crate::util::IdBufferRef;
use crate::Indexable;
use std::fmt::{Debug, Display};

///
/// A mutable reference to a gate inside a global buffer.
///
pub type GateRef = IdBufferRef<Gate>;

create_global_uid!(
    /// A runtime-unquie identifier for a gate.
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub GateId(u32) = GATE_ID;
);

///
/// A description of a gate / gate cluster on a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GateDescription {
    /// The identifier of the module the gate was created on.
    pub owner: ModuleId,
    /// A human readable name for a gate cluster.
    pub name: String,
    /// The number of elements in the gate cluster.
    pub size: usize,
}

impl GateDescription {
    ///
    /// Indicator whether a descriptor describes a cluster
    /// or a single gate
    ///
    #[inline(always)]
    pub fn is_cluster(&self) -> bool {
        self.size != 1
    }

    ///
    /// Creates a new descriptor using explicit values.
    ///
    pub fn new(name: String, size: usize, owner: ModuleId) -> Self {
        assert!(size >= 1);
        Self { name, size, owner }
    }
}

impl Display for GateDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

///
/// A gate, a message insertion or extraction point used for handeling channels.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Gate {
    /// A globally unique identifier for the gate.
    id: GateId,
    /// A descriptor of the cluster this gate belongs to.
    description: GateDescription,
    /// The position index of the gate in the descriptor cluster.
    pos: usize,
    /// A identifier of the channel linked to the gate chain.
    channel: Option<ChannelRef>,
    /// The next gate in the gate chain, GATE_NULL if non is existent.
    next_gate: GateId,
}

impl Indexable for Gate {
    type Id = GateId;

    fn id(&self) -> GateId {
        self.id
    }
}

impl Gate {
    /// The position index of the gate in the descriptor cluster.
    #[inline(always)]
    pub fn pos(&self) -> usize {
        self.pos
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.description.size
    }

    /// The human-readable name for the allocated gate cluster.
    #[inline(always)]
    pub fn name(&self) -> &String {
        &self.description.name
    }

    /// The next gate in the gate chain.
    #[inline(always)]
    pub fn next_gate(&self) -> GateId {
        self.next_gate
    }

    ///
    /// A function to link the next gate in the gate chain, by referencing
    /// its identifier.
    ///
    #[inline(always)]
    pub fn set_next_gate(&mut self, next_gate: GateId) {
        self.next_gate = next_gate;
    }

    /// The channel identifier of the linked channel.
    #[inline(always)]
    pub fn channel_id(&self) -> ChannelId {
        match &self.channel {
            Some(c) => c.id(),
            None => ChannelId::NULL,
        }
    }

    pub fn channel(&self) -> Option<&Channel> {
        match &self.channel {
            Some(c) => Some(c.get()),
            None => None,
        }
    }

    #[inline(always)]
    pub fn set_channel(&mut self, channel: ChannelRef) {
        self.channel = Some(channel)
    }

    /// The module idefnifier of the owner module.
    #[inline(always)]
    pub fn module(&self) -> ModuleId {
        self.description.owner
    }

    ///
    /// Creats a new gate using the given values.
    ///
    pub fn new(
        description: GateDescription,
        pos: usize,
        channel: Option<ChannelRef>,
        next_gate: GateId,
    ) -> Self {
        Self {
            id: GateId::gen(),
            description,
            pos,
            channel,
            next_gate,
        }
    }
}

///
/// A trait for a type to refrence a module specific gate.
///
pub trait IntoModuleGate<T>: Sized
where
    T: StaticModuleCore,
{
    ///
    /// Extracts a gate identifier from a module using the given
    /// value as implicit reference.
    ///
    fn into_gate(self, _module: &T) -> Option<GateId> {
        None
    }
}

impl<T> IntoModuleGate<T> for Gate
where
    T: StaticModuleCore,
{
    fn into_gate(self, module: &T) -> Option<GateId> {
        let element = module
            .gates()
            .iter()
            .find(|&g| **g == self)
            .map(|v| v.id())?;
        Some(element)
    }
}

impl<T> IntoModuleGate<T> for &Gate
where
    T: StaticModuleCore,
{
    fn into_gate(self, module: &T) -> Option<GateId> {
        let element = module
            .gates()
            .iter()
            .find(|&g| *g == self)
            .map(|v| v.id())?;
        Some(element)
    }
}

impl<T> IntoModuleGate<T> for GateId
where
    T: StaticModuleCore,
{
    fn into_gate(self, module: &T) -> Option<GateId> {
        let element = module
            .gates()
            .iter()
            .find(|&g| g.id() == self)
            .map(|v| v.id())?;
        Some(element)
    }
}

impl<T> IntoModuleGate<T> for (&str, usize)
where
    T: StaticModuleCore,
{
    fn into_gate(self, module: &T) -> Option<GateId> {
        let element = module
            .gates()
            .iter()
            .find(|&g| g.name() == self.0 && g.pos() == self.1)
            .map(|v| v.id())?;

        Some(element)
    }
}
