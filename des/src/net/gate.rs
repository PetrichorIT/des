//! Module-specific network ports.

use crate::net::channel::ChannelRef;
use std::cell::RefCell;
use std::fmt::Debug;
use std::sync::{Arc, Weak};

use super::module::{ModuleContext, ModuleRef, ModuleRefWeak};

///
/// A  reference to a gate.
///
pub type GateRef = Arc<Gate>;
///
/// A weak reference to a gate.
///
pub(crate) type GateRefWeak = Weak<Gate>;

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
/// A gate, a message insertion or extraction point used for handeling channels.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub struct Gate {
    owner: ModuleRefWeak,
    name: String,

    typ: GateServiceType,
    size: usize,
    pos: usize,

    channel: RefCell<Option<ChannelRef>>,

    next_gate: RefCell<Option<GateRef>>,
    previous_gate: RefCell<Option<GateRefWeak>>,
}

impl Gate {
    ///
    /// Indicator whether a descriptor describes a cluster
    /// or a single gate
    ///
    #[must_use]
    pub fn is_cluster(&self) -> bool {
        self.size != 1
    }

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
        self.size
    }

    ///
    /// The human-readable name for the allocated gate cluster.
    ///
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    fn name_with_pos(&self) -> String {
        if self.is_cluster() {
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
        self.typ
    }

    ///
    /// Returns a short identifcator that holds all nessecary information.
    ///
    #[must_use]
    pub fn str(&self) -> String {
        match self.typ {
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
        format!("{}:{}", self.owner().ctx.path, self.name_with_pos())
    }

    ///
    /// The next gate in the gate chain by reference.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
    ///
    #[must_use]
    pub fn previous_gate(&self) -> Option<GateRef> {
        self.previous_gate.borrow().clone()?.upgrade()
    }

    ///
    /// The next gate in the gate chain by reference.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
    ///
    #[must_use]
    pub fn next_gate(&self) -> Option<GateRef> {
        self.next_gate.borrow().clone()
    }

    ///
    /// A function to link the next gate in the gate chain, by referencing
    /// its identifier.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
    ///
    pub fn set_next_gate(self: &GateRef, next_gate: GateRef) {
        *next_gate.previous_gate.borrow_mut() = Some(Arc::downgrade(self));
        *self.next_gate.borrow_mut() = Some(next_gate);
    }

    ///
    /// Returns the channel attached to this gate, if any exits.
    ///
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
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
    /// # Panics
    ///
    /// Panics if the simulation core was poisoned.
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
    /// # Panics
    ///
    /// May panic when called in Drop, since the owner may allready
    /// be dropped.
    ///
    #[must_use]
    pub fn owner(&self) -> ModuleRef {
        self.owner.upgrade().unwrap()
    }

    ///
    /// Creats a new gate using the given values.
    ///
    /// # Panics
    ///
    /// Panics if the provided size is not real positive.
    #[must_use]
    pub fn new(
        owner: &ModuleRef,
        name: impl AsRef<str>,
        typ: GateServiceType,
        size: usize,
        pos: usize,
        channel: Option<ChannelRef>,
        next_gate: Option<GateRef>,
    ) -> GateRef {
        assert!(size >= 1, "Cannot create with a non-postive size");

        let this = GateRef::new(Self {
            owner: ModuleRefWeak::new(owner),
            name: name.as_ref().to_string(),
            typ,
            size,
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

impl Debug for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gate")
            .field("path", &self.path())
            .field("typ", &self.typ)
            // .field("next", &self.next_gate.borrow().as_ref().map(|_| ()))
            // .field("prev", &self.previous_gate.borrow().as_ref().map(|_| ()))
            // .field("channel", &self.channel.borrow())
            .finish()
    }
}

// SOLVED ISSUE: stack overflow when comaring circular ptr
// next_gate & previous_gate --> Custim PartialEq impl
impl PartialEq for Gate {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.owner.upgrade().unwrap().ctx.id == other.owner.upgrade().unwrap().ctx.id
            && self.size == other.size
            && self.pos == other.pos
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
