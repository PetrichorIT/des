use crate::net::ChannelRef;
use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::sync::{Arc, Weak};

use super::module::ModuleContext;
use super::ModuleRef;

///
/// A  reference to a gate.
///
pub type GateRef = Arc<Gate>;
///
/// A weak reference to a gate.
///
pub type GateRefWeak = Weak<Gate>;

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
    pub owner: ModuleRef,
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

    #[must_use]
    pub fn is_cluster(&self) -> bool {
        self.size != 1
    }

    ///
    /// Creates a new descriptor using explicit values and a service type.
    ///
    /// # Panics
    ///
    /// Panics if the size is 0.
    ///
    #[must_use]
    pub fn new(name: String, size: usize, owner: ModuleRef, typ: GateServiceType) -> Self {
        assert!(size >= 1, "Cannot create with a non-postive size");
        Self {
            owner,
            name,
            size,
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
            .field("owner", &self.owner.ctx.path)
            .field("typ", &self.typ)
            .finish()
    }
}

impl PartialEq for GateDescription {
    fn eq(&self, other: &Self) -> bool {
        // self.size can be ignored since no descriptors with the same name can exist
        // on the same owner
        self.name == other.name && self.owner.ctx.id == other.owner.ctx.id
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
    channel: RefCell<Option<ChannelRef>>,

    ///
    /// The next gate in the gate chain, GATE_NULL if non is existent.
    ///
    next_gate: RefCell<Option<GateRef>>,
    previous_gate: RefCell<Option<GateRefWeak>>,
}

impl Gate {
    ///
    /// The position index of the gate within the descriptor cluster.
    ///
    #[must_use]
    pub fn pos(&self) -> usize {
        self.pos
    }

    ///
    /// The size of the gate cluster.
    ///
    #[must_use]
    pub fn size(&self) -> usize {
        self.description.size
    }

    ///
    /// The human-readable name for the allocated gate cluster.
    ///
    #[must_use]
    pub fn name(&self) -> &str {
        &self.description.name
    }

    fn name_with_pos(&self) -> String {
        if self.description.is_cluster() {
            format!("{}[{}]", self.name(), self.pos())
        } else {
            self.name().to_string()
        }
    }

    ///
    /// Returns the serivce type of the gate cluster.
    ///
    #[must_use]
    pub fn service_type(&self) -> GateServiceType {
        self.description.typ
    }

    ///
    /// Returns a short identifcator that holds all nessecary information.
    ///
    #[must_use]
    pub fn str(&self) -> String {
        match self.description.typ {
            GateServiceType::Input => format!("{} (input)", self.name_with_pos()),
            GateServiceType::Output => format!("{} (output)", self.name_with_pos()),
            GateServiceType::Undefined => self.name_with_pos(),
        }
    }

    ///
    /// The full tree path of the gate.
    ///
    #[must_use]
    pub fn path(&self) -> String {
        format!(
            "{}:{}",
            self.description.owner.ctx.path,
            self.name_with_pos()
        )
    }

    ///
    /// The next gate in the gate chain by reference.
    ///
    #[must_use]
    pub fn previous_gate(&self) -> Option<GateRef> {
        self.previous_gate.borrow().clone()?.upgrade()
    }

    ///
    /// The next gate in the gate chain by reference.
    ///
    #[must_use]
    pub fn next_gate(&self) -> Option<GateRef> {
        self.next_gate.borrow().clone()
    }

    ///
    /// A function to link the next gate in the gate chain, by referencing
    /// its identifier.
    ///
    pub fn set_next_gate(self: &GateRef, next_gate: GateRef) {
        *next_gate.previous_gate.borrow_mut() = Some(Arc::downgrade(self));
        *self.next_gate.borrow_mut() = Some(next_gate);
    }

    ///
    /// Returns the channel attached to this gate, if any exits.
    ///
    #[must_use]
    pub fn channel(&self) -> Option<ChannelRef> {
        // only provide a read_only interface publicly
        Some(Arc::clone(self.channel.borrow().as_ref()?))
    }

    ///
    /// Returns the channel attached to this gate, if any exits.
    ///
    pub(crate) fn channel_mut(&self) -> Option<ChannelRef> {
        // only provide a read_only interface publicly
        Some(Arc::clone(self.channel.borrow().as_ref()?))
    }

    ///
    /// Sets the channel attached to this gate.
    ///
    pub fn set_channel(&self, channel: ChannelRef) {
        *self.channel.borrow_mut() = Some(channel);
    }

    ///
    /// Follows the previous-gate references until a gate without a previous-gate
    /// was found.
    ///
    #[must_use]
    pub fn path_start(&self) -> Option<GateRef> {
        let mut current = self.previous_gate()?;
        while let Some(previous_gate) = current.previous_gate() {
            current = previous_gate;
        }

        Some(current)
    }

    ///
    /// Follows the next-gate references until a gate without a next-gate
    /// was found.
    ///
    #[must_use]
    pub fn path_end(&self) -> Option<GateRef> {
        let mut current = self.next_gate()?;
        while let Some(next_gate) = current.next_gate() {
            current = next_gate;
        }

        Some(current)
    }

    ///
    /// Returns the owner module by reference of this gate.
    ///

    #[must_use]
    pub fn owner(&self) -> &ModuleRef {
        &self.description.owner
    }

    ///
    /// Creats a new gate using the given values.
    ///
    #[must_use]
    pub fn new(
        description: GateDescription,
        pos: usize,
        channel: Option<ChannelRef>,
        next_gate: Option<GateRef>,
    ) -> GateRef {
        let this = GateRef::new(Self {
            description,
            pos,
            channel: RefCell::new(channel),
            next_gate: RefCell::new(None),
            previous_gate: RefCell::new(None),
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
    fn as_gate(&self, _module: &ModuleContext) -> Option<GateRef> {
        None
    }
}

impl IntoModuleGate for GateRef {
    fn as_gate(&self, _module: &ModuleContext) -> Option<GateRef> {
        Some(self.clone())
    }
}
impl private::Sealed for GateRef {}

impl IntoModuleGate for &GateRef {
    fn as_gate(&self, _module: &ModuleContext) -> Option<GateRef> {
        Some(GateRef::clone(self))
    }
}
impl private::Sealed for &GateRef {}

impl IntoModuleGate for &GateRefWeak {
    fn as_gate(&self, _module: &ModuleContext) -> Option<GateRef> {
        self.upgrade()
    }
}
impl private::Sealed for &GateRefWeak {}

impl IntoModuleGate for (&str, usize) {
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef> {
        module
            .gates
            .borrow()
            .iter()
            .find(|&g| g.name() == self.0 && g.pos() == self.1)
            .cloned()
    }
}
impl private::Sealed for (&str, usize) {}

impl IntoModuleGate for &str {
    fn as_gate(&self, module: &ModuleContext) -> Option<GateRef> {
        module
            .gates
            .borrow()
            .iter()
            .find(|&g| g.name() == *self && g.size() == 1)
            .cloned()
    }
}
impl private::Sealed for &str {}
