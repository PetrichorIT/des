use crate::net::*;
use crate::util::*;
use std::fmt::{Debug, Display};
use std::marker::Unsize;

use super::module::ModuleCore;

///
/// A readonly reference to a gate.
///
pub type GateRef = PtrConst<Gate>;

///
/// A mutable reference to a gate.
///
pub type GateRefMut = PtrMut<Gate>;

///
/// The type of service a gate cluster can support.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateServiceType {
    /// A gate that can be connected to in NDL notation
    Input,
    /// A gate that can be pointed to antoher gate in NDL notation.
    Output,
    /// A gate without restrictions.
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
    pub owner: PtrWeakMut<dyn Module>,
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
    #[must_use]
    pub fn new<T>(name: String, size: usize, owner: PtrWeakMut<T>, typ: GateServiceType) -> Self
    where
        T: Module + Unsize<dyn Module>,
    {
        let owner: PtrWeakMut<dyn Module> = owner;
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
#[derive(Debug, Clone)]
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
    previous_gate: Option<GateRef>,
}

impl Gate {
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
    pub fn previous_gate(&self) -> Option<&GateRef> {
        self.previous_gate.as_ref()
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
    pub fn set_next_gate(self: &mut PtrMut<Self>, mut next_gate: GateRefMut) {
        self.next_gate = Some(PtrMut::clone(&next_gate).make_const());
        next_gate.previous_gate = Some(self.clone().make_const());
    }

    ///
    /// Returns the channel attached to this gate, if any exits.
    ///
    pub fn channel(&self) -> Option<ChannelRef> {
        // only provide a read_only interface publicly
        Some(Ptr::clone(self.channel.as_ref()?).make_const())
    }

    ///
    /// Returns the channel attached to this gate, if any exits.
    ///
    pub(crate) fn channel_mut(&self) -> Option<ChannelRefMut> {
        // only provide a read_only interface publicly
        Some(Ptr::clone(self.channel.as_ref()?))
    }

    ///
    /// Sets the channel attached to this gate.
    ///
    #[inline(always)]
    pub fn set_channel(&mut self, channel: ChannelRefMut) {
        self.channel = Some(channel)
    }

    ///
    /// Follows the previous-gate references until a gate without a previous-gate
    /// was found.
    ///
    pub fn path_start(&self) -> Option<GateRef> {
        let mut current = self.previous_gate.as_ref()?;
        while let Some(previous_gate) = &current.previous_gate {
            current = previous_gate
        }

        Some(PtrConst::clone(current))
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

        Some(PtrConst::clone(current))
    }

    ///
    /// Returns the owner module by reference of this gate.
    ///
    #[inline(always)]
    pub fn owner(&self) -> &PtrWeakMut<dyn Module> {
        &self.description.owner
    }

    ///
    /// Creats a new gate using the given values.
    ///
    #[must_use]
    pub fn new(
        description: GateDescription,
        pos: usize,
        channel: Option<ChannelRefMut>,
        next_gate: Option<GateRefMut>,
    ) -> GateRefMut {
        let mut this = PtrMut::new(Self {
            description,
            pos,
            channel,
            next_gate: None,
            previous_gate: None,
        });

        if let Some(next_gate) = next_gate {
            this.set_next_gate(next_gate);
        }
        this
    }
}

// SAFTY:
// Gates are never exposed by value to the user so they will be marked
// as `Send` to fulfill the trait bound for Ptr<Gate> to be `Send`.
//
unsafe impl Send for Gate {}

// SOLVED ISSUE: stack overflow when comaring circular ptr
// next_gate & previous_gate --> Custim PartialEq impl
impl PartialEq for Gate {
    fn eq(&self, other: &Self) -> bool {
        self.description == other.description && self.pos == other.pos
    }
}
impl Eq for Gate {}

mod private {
    pub trait Sealed {}
}

///
/// A trait for a type to refrence a module specific gate.
///
pub trait IntoModuleGate: private::Sealed {
    ///
    /// Extracts a gate identifier from a module using the given
    /// value as implicit reference.
    ///
    fn as_gate(&self, _module: &ModuleCore) -> Option<GateRef> {
        None
    }
}

impl IntoModuleGate for GateRef {
    fn as_gate(&self, _module: &ModuleCore) -> Option<GateRef> {
        Some(self.clone())
    }
}
impl private::Sealed for GateRef {}

impl IntoModuleGate for &GateRef {
    fn as_gate(&self, _module: &ModuleCore) -> Option<GateRef> {
        Some(Ptr::clone(self))
    }
}
impl private::Sealed for &GateRef {}

impl IntoModuleGate for GateRefMut {
    fn as_gate(&self, _module: &ModuleCore) -> Option<GateRef> {
        Some(self.clone().make_const())
    }
}
impl private::Sealed for GateRefMut {}

impl IntoModuleGate for &GateRefMut {
    fn as_gate(&self, _module: &ModuleCore) -> Option<GateRef> {
        Some(Ptr::clone(self).make_const())
    }
}
impl private::Sealed for &GateRefMut {}

impl IntoModuleGate for (&str, usize) {
    fn as_gate(&self, module: &ModuleCore) -> Option<GateRef> {
        let element = module
            .gates()
            .iter()
            .find(|&g| g.name() == self.0 && g.pos() == self.1)?;

        Some(Ptr::clone(element).make_const())
    }
}
impl private::Sealed for (&str, usize) {}

impl IntoModuleGate for &str {
    fn as_gate(&self, module: &ModuleCore) -> Option<GateRef> {
        let element = module
            .gates()
            .iter()
            .find(|&g| g.name() == *self && g.size() == 1)?;

        Some(Ptr::clone(element).make_const())
    }
}
impl private::Sealed for &str {}
