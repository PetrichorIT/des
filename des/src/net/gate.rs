use crate::net::*;
use crate::util::mm::Mrc;
use std::fmt::{Debug, Display};
use std::marker::Unsize;

///
/// A mutable reference to a gate inside a global buffer.
///
pub type GateRef = Mrc<Gate>;

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
    pub owner: ModuleRef,
    ///
    /// A human readable name for a gate cluster.
    ///
    pub name: String,
    ///
    /// The number of elements in the gate cluster.
    ///
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
    pub fn new<T>(name: String, size: usize, owner: Mrc<T>) -> Self
    where
        T: Module + Unsize<dyn Module>,
    {
        let owner: Mrc<dyn Module> = owner;
        assert!(size >= 1);
        Self { name, size, owner }
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
    channel: Option<ChannelRef>,
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
        Some(Mrc::clone(self.channel.as_ref()?))
    }

    ///
    /// Sets the channel attached to this gate.
    ///
    #[inline(always)]
    pub fn set_channel(&mut self, channel: ChannelRef) {
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

        Some(Mrc::clone(current))
    }

    ///
    /// Returns the owner module by reference of this gate.
    ///
    #[inline(always)]
    pub fn owner(&self) -> &ModuleRef {
        &self.description.owner
    }

    ///
    /// Creats a new gate using the given values.
    ///
    pub fn new(
        description: GateDescription,
        pos: usize,
        channel: Option<ChannelRef>,
        next_gate: Option<GateRef>,
    ) -> Mrc<Self> {
        Mrc::new(Self {
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
pub trait IntoModuleGate<T>: Sized
where
    T: StaticModuleCore,
{
    ///
    /// Extracts a gate identifier from a module using the given
    /// value as implicit reference.
    ///
    fn into_gate(self, _module: &T) -> Option<GateRef> {
        None
    }
}

impl<T> IntoModuleGate<T> for GateRef
where
    T: StaticModuleCore,
{
    fn into_gate(self, _module: &T) -> Option<GateRef> {
        Some(self)
    }
}

impl<T> IntoModuleGate<T> for &GateRef
where
    T: StaticModuleCore,
{
    fn into_gate(self, _module: &T) -> Option<GateRef> {
        Some(Mrc::clone(self))
    }
}

impl<T> IntoModuleGate<T> for (&str, usize)
where
    T: StaticModuleCore,
{
    fn into_gate(self, module: &T) -> Option<GateRef> {
        let element = module
            .gates()
            .iter()
            .find(|&g| g.name() == self.0 && g.pos() == self.1)?;

        Some(Mrc::clone(element))
    }
}
