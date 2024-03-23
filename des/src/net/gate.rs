//! Module-specific network ports.

use crate::net::channel::ChannelRef;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, Weak};

use super::module::{ModuleContext, ModuleRef, ModuleRefWeak};
use super::ObjectPath;

///
/// A  reference to a gate.
///
pub type GateRef = Arc<Gate>;
///
/// A weak reference to a gate.
///
pub(crate) type GateRefWeak = Weak<Gate>;

///
/// A gate, a message insertion or extraction point used for handeling channels.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub struct Gate {
    owner: ModuleRefWeak,
    name: String,

    size: usize,
    pos: usize,

    connections: Mutex<Connections>,
}

/// A kinds of operations supported on a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateKind {
    /// Standalone gates are not connected to any gate chain
    /// at all. They act as start and endpoint of a gatechain of
    /// length 0. Messages send onto these gates will never leave
    /// the sending module.
    Standalone,
    /// Endpoint gates are at the start or end of gate chains.
    /// These gates can be used to send messages onto a gate chain.
    /// Each endpoint acts as the starting point for one direction.
    Endpoint,
    /// Transit gates are in the middle of a gate chain, connected
    /// to two other gates. These gates cannot be used to start
    /// a message sending process.
    Transit,
}

struct Connections {
    connections: [Option<Connection>; 2],
}

/// A connection to a peering gate
#[derive(Debug, Clone)]
pub struct Connection {
    /// The endpoint from the view of the owning gate
    pub endpoint: GateRef,
    /// The index of the slot used at the endpoint
    pub endpoint_id: usize,
    /// A channel to slow down the connection.
    pub channel: Option<ChannelRef>,
}

impl Connection {
    /// Crease a new pseudo connection, channeling into the
    /// provided gate
    ///
    /// # Panics
    ///
    /// Panics if the provided gate is not an endpoint.
    pub fn new(gate: GateRef) -> Self {
        assert!(
            gate.connections
                .lock()
                .expect("locking failure: GateRef seems to be active on another thread")
                .len()
                <= 1
        );
        Self::new_unchecked(gate)
    }

    ///
    pub fn new_unchecked(gate: GateRef) -> Self {
        Self {
            endpoint: gate,
            endpoint_id: 1, // TODO: smarter ??
            channel: None,
        }
    }

    /// NEXT
    ///
    /// # Panics
    ///
    /// May panic on lock poisoning
    #[must_use]
    pub fn next_hop(&self) -> Option<Connection> {
        let idx = [1, 0][self.endpoint_id];
        let lock = self
            .endpoint
            .connections
            .lock()
            .expect("failed to get lock");
        lock.connections[idx].clone()
    }

    /// PREV
    /// # Panics
    ///
    /// May panic on lock poisoning
    #[must_use]
    pub fn prev_hop(&self) -> Option<GateRef> {
        let lock = self
            .endpoint
            .connections
            .lock()
            .expect("failed to get lock");
        Some(
            lock.connections[self.endpoint_id]
                .as_ref()?
                .endpoint
                .clone(),
        )
    }

    /// CHAN
    pub fn channel(&self) -> Option<ChannelRef> {
        self.channel.as_ref().map(Arc::clone)
    }
}

impl Connections {
    fn new() -> Self {
        Self {
            connections: [None, None],
        }
    }

    fn len(&self) -> usize {
        self.connections.iter().filter(|v| v.is_some()).count()
    }

    fn put(&mut self, connection: Connection) {
        for i in 0..2 {
            if self.connections[i].is_none() {
                self.connections[i] = Some(connection);
                return;
            }
        }
        unreachable!()
    }
}

struct PathIter {
    con: Option<Connection>,
}

impl Iterator for PathIter {
    type Item = Connection;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.con.as_ref()?.next_hop()?;
        self.con = Some(next.clone());
        Some(next)
    }
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
    /// Returns a short identifcator that holds all nessecary information.
    ///
    #[must_use]
    pub fn str(&self) -> String {
        self.name_with_pos()
    }

    ///
    /// The full tree path of the gate.
    ///
    #[must_use]
    pub fn path(&self) -> ObjectPath {
        self.owner().ctx.path.appended_gate(self.name_with_pos())
    }

    /// Returns the kind of operations allowed on this gate.
    ///
    /// # Panics
    ///
    /// Panics when accessed during teardown
    pub fn kind(&self) -> GateKind {
        match self
            .connections
            .try_lock()
            .expect("failed to get lock")
            .len()
        {
            0 => GateKind::Standalone,
            1 => GateKind::Endpoint,
            _ => GateKind::Transit,
        }
    }

    /// Connects two gates into a gate chain element.
    ///
    /// Gates can be organized into a bidirectional gate chain, that
    /// forwards messages two the other end. Using this function two gates
    /// are connected and both gates save their connection state. A gate
    /// can have up to two other gates connected to it, forming a full gate
    /// chain in response.
    ///
    /// If a channel was provided to enable message delaying on this chain element
    /// both direction will have unique instances of the channel, with identical
    /// configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # fn a() -> Option<()>{
    /// # return None;
    /// let a = current().gate("out", 0)?;
    /// let b = current().parent().ok()?.gate("in", 0)?;
    /// a.connect(b, None);
    /// # Some(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// This function panic if either of the two gates is allready fully connected in a chain.
    /// This function also panics if only one gate is provided
    #[allow(clippy::needless_pass_by_value)]
    pub fn connect(self: GateRef, other: GateRef, channel: Option<ChannelRef>) {
        assert!(
            !Arc::ptr_eq(&self, &other),
            "Cannot connect gate to itself."
        );

        let mut conns = self.connections.try_lock().expect("Failed to get lock");
        let mut other_conns = other.connections.try_lock().expect("failed to get lock");

        let conns_pos = conns.len();
        let other_conns_pos = other_conns.len();
        assert!(
            conns_pos < 2 && other_conns_pos < 2,
            "Cannot add connection, gates allready connected to multiple points"
        );

        let ch1 = channel.as_ref().map(|c| Arc::new(c.dup()));
        let ch2 = channel;

        conns.put(Connection {
            endpoint: other.clone(),
            endpoint_id: other_conns_pos,
            channel: ch1,
        });
        other_conns.put(Connection {
            endpoint: self.clone(),
            endpoint_id: conns_pos,
            channel: ch2,
        });
    }

    /// DEDUP
    /// # Panics
    ///
    /// May panic when accessed during teardown
    pub fn connect_dedup(self: GateRef, other: GateRef, channel: Option<ChannelRef>) {
        // Check whether the target is allready connected
        let conns = self.connections.try_lock().expect("failed lock");
        for i in 0..2 {
            if let Some(ref con) = conns.connections[i] {
                if Arc::ptr_eq(&con.endpoint, &other) {
                    return;
                }
            }
        }

        drop(conns);
        self.connect(other, channel);
    }

    /// CHAN
    pub fn channel(self: &GateRef) -> Option<ChannelRef> {
        self.path_iter().nth(0).and_then(|con| con.channel)
    }

    /// ITER
    pub fn path_iter(self: &GateRef) -> impl Iterator<Item = Connection> {
        PathIter {
            con: Some(Connection::new_unchecked(self.clone())),
        }
    }

    /// NEXT GATE
    pub fn next_gate(self: &GateRef) -> Option<GateRef> {
        self.path_iter().nth(0).map(|c| c.endpoint)
    }

    /// END
    pub fn path_end(self: &GateRef) -> Option<GateRef> {
        self.path_iter().last().map(|c| c.endpoint)
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
        self.owner
            .upgrade()
            .expect("cannot refer to gate owner during drop")
    }

    ///
    /// Creats a new gate using the given values.
    ///
    /// # Panics
    ///
    /// Panics if the provided size is not real positive.
    #[must_use]
    pub fn new(owner: &ModuleRef, name: impl AsRef<str>, size: usize, pos: usize) -> GateRef {
        assert!(size >= 1, "Cannot create with a non-postive size");

        let this = GateRef::new(Self {
            owner: ModuleRefWeak::new(owner),
            name: name.as_ref().to_string(),
            size,
            pos,
            connections: Mutex::new(Connections::new()),
        });

        this
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gate")
            .field("path", &self.path())
            // .field("typ", &self.typ)
            // .field("next", &self.next_gate.borrow().as_ref().map(|_| ()))
            // .field("prev", &self.previous_gate.borrow().as_ref().map(|_| ()))
            // .field("channel", &self.channel.borrow())
            .finish()
    }
}

// SAFTY:
// Gates are never exposed by value to the user so they will be marked
// as `Send` to fulfill the trait bound for Ptr<Gate> to be `Send`.
unsafe impl Send for Gate {}

// SOLVED ISSUE: stack overflow when comaring circular ptr
// next_gate & previous_gate --> Custim PartialEq impl
impl PartialEq for Gate {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.owner().ctx.id == other.owner().ctx.id
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
            .read()
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
            .read()
            .iter()
            .find(|&g| g.name() == *self && g.size() == 1)
            .cloned()
    }
}
impl private::Sealed for &str {}
