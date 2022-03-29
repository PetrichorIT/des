use crate::net::*;
use crate::util::{MrcS, Mutable, ReadOnly};
use std::fmt::{Debug, Display};
use std::marker::Unsize;

///
/// A readonly reference to a gate.
///
pub type GateRef = MrcS<Gate, ReadOnly>;

///
/// A mutable reference to a gate.
///
pub type GateRefMut = MrcS<Gate, Mutable>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateServiceType {
    Input,
    Output,
    Undefined,
}

///
/// A description of a gate / gate cluster on a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Clone)]
pub struct GateDescription {
    ///
    /// The identifier of the module the gate was created on.
    ///
    pub owner: ModuleRefMut,
    ///
    /// A human readable name for a gate cluster.
    ///
    pub name: String,

    ///
    /// The number of elements in the gate cluster.
    ///
    pub size: usize,

    ///
    /// The service type of the given gate.
    ///
    pub typ: GateServiceType,
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
    /// Creates a new descriptor using explicit values and a service type.
    ///
    pub fn new<T>(name: String, size: usize, owner: MrcS<T, Mutable>, typ: GateServiceType) -> Self
    where
        T: Module + Unsize<dyn Module>,
    {
        let owner: ModuleRefMut = owner;
        assert!(size >= 1, "Cannot create with a non-postive size");
        Self {
            name,
            size,
            owner,
            typ,
        }
    }
}

impl Display for GateDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl Debug for GateDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GateDescription")
            .field("name", &self.name)
            .field("size", &self.size)
            .field("owner", self.owner.path())
            .field("typ", &self.typ)
            .finish()
    }
}

impl PartialEq for GateDescription {
    fn eq(&self, other: &Self) -> bool {
        // self.size can be ignored since no descriptors with the same name can exist
        // on the same owner
        self.name == other.name && self.owner.id() == other.owner.id()
    }
}

impl Eq for GateDescription {}

///
/// A gate, a message insertion or extraction point used for handeling channels.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    ///
    /// A descriptor of the cluster this gate belongs to.
    ///
    description: GateDescription,
    ///
    /// The position index of the gate in the descriptor cluster.
    ///
    pos: usize,
    ///
    /// A identifier of the channel linked to the gate chain.
    ///
    channel: Option<ChannelRefMut>,
    ///
    /// The next gate in the gate chain, GATE_NULL if non is existent.
    ///
    next_gate: Option<GateRef>,
}

impl Gate {
    #[deprecated(since = "0.2.0", note = "GateIDs are no longer supported")]
    pub fn id(&self) -> ! {
        unimplemented!("GateIDs are no longer supported");
    }

    ///
    /// The position index of the gate within the descriptor cluster.
    ///
    #[inline(always)]
    pub fn pos(&self) -> usize {
        self.pos
    }

    ///
    /// The size of the gate cluster.
    ///
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.description.size
    }

    ///
    /// The human-readable name for the allocated gate cluster.
    ///
    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.description.name
    }

    ///
    /// Returns the serivce type of the gate cluster.
    ///
    pub fn service_type(&self) -> GateServiceType {
        self.description.typ
    }

    ///
    /// Returns a short identifcator that holds all nessecary information.
    ///
    pub fn str(&self) -> String {
        match self.description.typ {
            GateServiceType::Input => format!("{} (input)", self.name()),
            GateServiceType::Output => format!("{} (output)", self.name()),
            _ => self.name().to_string(),
        }
    }

    ///
    /// The full tree path of the gate.
    ///
    pub fn path(&self) -> String {
        format!("{}:{}", self.description.owner.path(), self.name())
    }

    ///
    /// The next gate in the gate chain by reference.
    ///
    #[inline(always)]
    pub fn next_gate(&self) -> Option<&GateRef> {
        self.next_gate.as_ref()
    }

    ///
    /// A function to link the next gate in the gate chain, by referencing
    /// its identifier.
    ///
    #[inline(always)]
    pub fn set_next_gate(&mut self, next_gate: GateRef) {
        self.next_gate = Some(next_gate);
    }

    ///
    /// Returns the channel attached to this gate, if any exits.
    ///
    pub fn channel(&self) -> Option<ChannelRef> {
        // only provide a read_only interface publicly
        Some(MrcS::clone(self.channel.as_ref()?).make_readonly())
    }

    ///
    /// Returns the channel attached to this gate, if any exits.
    ///
    pub(crate) fn channel_mut(&self) -> Option<ChannelRefMut> {
        // only provide a read_only interface publicly
        Some(MrcS::clone(self.channel.as_ref()?))
    }

    ///
    /// Sets the channel attached to this gate.
    ///
    #[inline(always)]
    pub fn set_channel(&mut self, channel: ChannelRefMut) {
        self.channel = Some(channel)
    }

    ///
    /// Follows the next-gate references until a gate without a next-gate
    /// was found.
    ///
    pub fn path_end(&self) -> Option<GateRef> {
        let mut current = self.next_gate.as_ref()?;
        while let Some(next_gate) = &current.next_gate {
            current = next_gate
        }

        Some(MrcS::clone(current))
    }

    ///
    /// Returns the owner module by reference of this gate.
    ///
    #[inline(always)]
    pub fn owner(&self) -> &ModuleRefMut {
        &self.description.owner
    }

    ///
    /// Creats a new gate using the given values.
    ///
    pub fn new(
        description: GateDescription,
        pos: usize,
        channel: Option<ChannelRefMut>,
        next_gate: Option<GateRef>,
    ) -> GateRefMut {
        MrcS::new(Self {
            description,
            pos,
            channel,
            next_gate,
        })
    }
}

///
/// A trait for a type to refrence a module specific gate.
///
pub trait IntoModuleGate: Sized {
    ///
    /// Extracts a gate identifier from a module using the given
    /// value as implicit reference.
    ///
    fn into_gate(self, _module: &ModuleCore) -> Option<GateRef> {
        None
    }
}

impl IntoModuleGate for GateRef {
    fn into_gate(self, _module: &ModuleCore) -> Option<GateRef> {
        Some(self)
    }
}

impl IntoModuleGate for &GateRef {
    fn into_gate(self, _module: &ModuleCore) -> Option<GateRef> {
        Some(MrcS::clone(self))
    }
}

impl IntoModuleGate for GateRefMut {
    fn into_gate(self, _module: &ModuleCore) -> Option<GateRef> {
        Some(self.make_readonly())
    }
}

impl IntoModuleGate for &GateRefMut {
    fn into_gate(self, _module: &ModuleCore) -> Option<GateRef> {
        Some(MrcS::clone(self).make_readonly())
    }
}

impl IntoModuleGate for (&str, usize) {
    fn into_gate(self, module: &ModuleCore) -> Option<GateRef> {
        let element = module
            .gates()
            .iter()
            .find(|&g| g.name() == self.0 && g.pos() == self.1)?;

        Some(MrcS::clone(element).make_readonly())
    }
}

impl IntoModuleGate for &str {
    fn into_gate(self, module: &ModuleCore) -> Option<GateRef> {
        let element = module
            .gates()
            .iter()
            .find(|&g| g.name() == self && g.size() == 1)?;

        Some(MrcS::clone(element).make_readonly())
    }
}
