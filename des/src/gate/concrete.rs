use des_sync_utils::RwLock;

use crate::channel::{ChannelRef, IntoDuplexChannel};
use crate::gate::{GateCluster, GateClusterRef, IntoGate};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, Weak};

use crate::ObjectPath;
use crate::module::{ModuleRef, ModuleRefWeak};

/// A  reference to a gate.
pub type GateRef = Arc<Gate>;
/// A weak reference to a gate.
pub(crate) type GateRefWeak = Weak<Gate>;

/// A gate, a message insertion or extraction point used for handeling channels.
pub struct Gate {
    owner: ModuleRefWeak,
    cluster: Weak<GateCluster>,
    name: String,
    pos: usize,
    connections: RwLock<Connections>,
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
    /// A channel to slow down the connection.
    pub channel: Option<ChannelRef>,

    /// The index of the slot used at the endpoint to refer to the inverse link
    pub(crate) endpoint_id: usize,
}

impl Connection {
    /// Creates a new sourceless connection that points to the provided gate.
    ///
    /// # Panics
    ///
    /// This function panics if the provided gate is of kind Transit, as it is
    /// not clear in what direction the connection-chain should point
    pub fn new_sourceless(endpoint: GateRef) -> Self {
        assert!(endpoint.connections.read().len() <= 1);
        // There will be at least one free slot
        let endpoint_id = endpoint.connections.read().next_free().unwrap_or(0);
        Self {
            endpoint,
            endpoint_id,
            channel: None,
        }
    }

    /// Retrives the next hop from the connection, in the direction of iteration.
    ///
    /// # Panics
    ///
    /// May panic on lock poisoning
    #[must_use]
    pub fn next_hop(&self) -> Option<Connection> {
        let idx = [1, 0][self.endpoint_id];
        let lock = self.endpoint.connections.read();
        lock.connections[idx].clone()
    }

    /// Retrives the previous hop from the connection, in the direction of iteration.
    ///
    /// # Panics
    ///
    /// May panic on lock poisoning
    #[must_use]
    pub fn prev_hop(&self) -> Option<GateRef> {
        let lock = self.endpoint.connections.read();
        Some(
            lock.connections[self.endpoint_id]
                .as_ref()?
                .endpoint
                .clone(),
        )
    }

    /// Retrieves a handle to a channel attached to this connection.
    #[must_use]
    pub fn channel(&self) -> Option<ChannelRef> {
        self.channel.clone()
    }

    fn unregister(self) {
        if let Some(channel) = self.channel {
            channel
                .channel
                .try_write()
                .expect("failed to get lock")
                .unregister(self.endpoint);
        }
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

    fn next_free(&self) -> Option<usize> {
        (0..2).find(|&i| self.connections[i].is_none())
    }

    fn put(&mut self, index: usize, connection: Connection) {
        assert!(
            self.connections[index].is_none(),
            "Connections::put should only be called with a valid / empty slot index, obtained by next_index()"
        );
        self.connections[index] = Some(connection);
    }

    fn remove(&mut self, gate: &GateRef) -> Connection {
        for i in 0..2 {
            if self.connections[i]
                .as_ref()
                .is_some_and(|c| Arc::ptr_eq(gate, &c.endpoint))
            {
                return self.connections[i].take().expect("illegal state");
            }
        }
        unreachable!("Connections::remove should not be called on unconnected elements")
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
    /// Indicator whether a descriptor describes a cluster
    /// or a single gate
    #[must_use]
    pub fn is_standalone(&self) -> bool {
        self.size() == 1 && self.pos() == 0
    }

    /// The position index of the gate within the descriptor cluster.
    #[must_use]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// The size of the gate cluster.
    #[must_use]
    pub fn size(&self) -> usize {
        self.cluster().size()
    }

    /// Retrives the cluster descriptor for this gate.
    ///
    /// # Panics
    ///
    /// This function panics when used during drop.
    #[must_use]
    pub fn cluster(&self) -> GateClusterRef {
        self.cluster.upgrade().expect("cannot use in drop")
    }

    /// The human-readable name for the allocated gate cluster.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    fn name_with_pos(&self) -> String {
        if self.is_standalone() {
            self.name().to_owned()
        } else {
            format!("{}[{}]", self.name(), self.pos())
        }
    }

    /// Returns a short identifcator that holds all nessecary information.
    #[must_use]
    pub fn str(&self) -> String {
        self.name_with_pos()
    }

    /// The full tree path of the gate.
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
        match self.connections.read().len() {
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
    /// let a = current().gate(("out", 0))?;
    /// let b = current().parent().ok()?.gate(("in", 0))?;
    /// a.connect(b);
    /// # Some(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// This function panic if either of the two gates is allready fully connected in a chain.
    /// This function also panics if only one gate is provided
    #[allow(clippy::needless_pass_by_value)]
    pub(super) fn raw_connect_with<C: IntoDuplexChannel>(
        self: GateRef,
        other: GateRef,
        channel: Option<C>,
    ) {
        assert!(
            !Arc::ptr_eq(&self, &other),
            "Cannot connect gate to itself."
        );

        // (0) Check whether the target is already connected
        let mut conns = self.connections.write();
        for i in 0..2 {
            if let Some(ref con) = conns.connections[i]
                && Arc::ptr_eq(&con.endpoint, &other)
            {
                return;
            }
        }

        // (1) Determine the valid slots in both gates, where the connection could be stored.
        let mut other_conns = other.connections.write();
        let conns_pos = conns
            .next_free()
            .expect("cannot add connection, gates already connected to multiple points");
        let other_conns_pos = other_conns
            .next_free()
            .expect("cannot add connection, gates already connected to multiple points");

        // (2) Register the channel(s) if required
        let (fwd, bck) = match channel {
            Some(channel) => {
                let (a, b) = channel.into_duplex();
                a.channel
                    .try_write()
                    .expect("failed to get lock")
                    .register(self.clone());
                b.channel
                    .try_write()
                    .expect("failed to get lock")
                    .register(other.clone());

                (Some(a), Some(b))
            }
            None => (None, None),
        };

        // (3) Store connection objects in the previously determined slots
        conns.put(
            conns_pos,
            Connection {
                endpoint: other.clone(),
                endpoint_id: other_conns_pos,
                channel: fwd,
            },
        );
        other_conns.put(
            other_conns_pos,
            Connection {
                endpoint: self.clone(),
                endpoint_id: conns_pos,
                channel: bck,
            },
        );
    }

    /// Disconnects a peer.
    ///
    /// After successful execution the two gates will no longer be connnected. If a channel exists
    /// on thus link, it will be informed via `Channel::unregister`.
    ///
    /// # Panics
    ///
    /// This function panics if the specified peer is not connected to this gate.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::module::Signal;
    /// # const SIGNAL_DISCONNECT: usize = 1231;
    /// struct MyModule {}
    ///
    /// impl Module for MyModule {
    ///     fn handle_signal(&mut self, signal: Signal) {
    ///         assert_eq!(signal.code, SIGNAL_DISCONNECT);
    ///         let gate = current().gate("uplink").expect("failed to get gate");
    ///         let next = gate.next_gate().expect("failed to get next");
    ///         gate.disconnect(&next);
    ///     }
    /// }
    /// ```
    pub fn disconnect(self: &GateRef, other: &GateRef) {
        assert!(
            self.is_neighbor_to(other),
            "cannot disconnect two unconnected gates"
        );

        let mut conns = self.connections.write();
        let mut other_conns = other.connections.write();

        let local_con = conns.remove(other);
        let peer_con = other_conns.remove(self);

        local_con.unregister();
        peer_con.unregister();
    }

    /// Disconnects all peers. This method cannot fail.
    #[allow(clippy::missing_panics_doc)]
    pub fn disconnect_all(self: &GateRef) {
        let mut conns = self.connections.write();
        for i in 0..2 {
            if let Some(local_con) = conns.connections[i].take() {
                let mut other_conns = local_con.endpoint.connections.write();

                let peer_con = other_conns.remove(self);
                peer_con.unregister();
                drop(other_conns);
                local_con.unregister();
            }
        }
    }

    /// Checks whether two gates are direct neighbors, works even for transit gates.
    #[allow(clippy::missing_panics_doc)]
    pub fn is_neighbor_to(self: &GateRef, other: &GateRef) -> bool {
        let conns = self.connections.read();
        for i in 0..2 {
            if conns.connections[i]
                .as_ref()
                .is_some_and(|c| Arc::ptr_eq(&c.endpoint, other))
            {
                return true;
            }
        }
        false
    }

    /// Retrives the channel of the first connection on the path.
    pub fn channel(self: &GateRef) -> Option<ChannelRef> {
        self.path_iter()?.nth(0).and_then(|con| con.channel)
    }

    /// Retrieves the next channel on the path.
    pub fn next_channel(self: &GateRef) -> Option<ChannelRef> {
        self.path_iter()?.find_map(|con| con.channel)
    }

    /// Returns an iterator over the connections on a gate path.
    /// If the current gate is a transit gate, no iterator will be returned,
    /// since the direction of the iterator cannot be determined.
    pub fn path_iter(self: &GateRef) -> Option<impl Iterator<Item = Connection> + use<>> {
        if self.kind() == GateKind::Transit {
            None
        } else {
            Some(PathIter {
                con: Some(Connection::new_sourceless(self.clone())),
            })
        }
    }

    /// Retrieves the next gate on the path.
    ///
    /// If the current gate is a not an endpoint, `None` will be returned.
    pub fn next_gate(self: &GateRef) -> Option<GateRef> {
        self.path_iter()?.nth(0).map(|c| c.endpoint)
    }

    /// Retrieves the last gate on the path.
    ///
    /// If the current gate is a not an endpoint, `None` will be returned.
    pub fn path_end(self: &GateRef) -> Option<GateRef> {
        self.path_iter()?.last().map(|c| c.endpoint)
    }

    /// Returns the owner module by reference of this gate.
    ///
    /// # Panics
    ///
    /// May panic when called in Drop, since the owner may allready
    /// be dropped.
    #[must_use]
    pub fn owner(&self) -> ModuleRef {
        self.owner
            .upgrade()
            .expect("cannot refer to gate owner during drop")
    }

    pub(super) fn raw(
        owner: ModuleRefWeak,
        cluster: &GateClusterRef,
        name: &str,
        pos: usize,
    ) -> GateRef {
        GateRef::new(Gate {
            owner,
            cluster: GateClusterRef::downgrade(cluster),
            name: name.to_owned(),
            pos,
            connections: RwLock::new(Connections::new()),
        })
    }

    /// Creats a new gate using the given values.
    ///
    /// # Panics
    ///
    /// Panics if the provided size is not real positive.
    #[must_use]
    pub fn new(owner: &ModuleRef, name: &str, pos: Option<usize>) -> GateRef {
        let mut handle = owner.gates.write();
        handle.create_gate(name, pos)
    }

    pub(crate) fn dissolve_paths(&self) {
        let Some(mut conns) = self.connections.try_write() else {
            return;
        };
        for con in &mut conns.connections {
            if let Some(con) = con.take() {
                con.endpoint.dissolve_paths();
            }
        }
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Custom impl to take only weak locks
        f.debug_struct("Gate")
            .field(
                "path",
                &format!("{}.{}[{}]", self.owner().path, self.name(), self.pos()),
            )
            .finish()
    }
}

// SOLVED ISSUE: stack overflow when comaring circular ptr
// next_gate & previous_gate --> Custim PartialEq impl
impl PartialEq for Gate {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && Arc::ptr_eq(&self.owner().ctx, &other.owner().ctx)
            && self.pos == other.pos
    }
}

impl Eq for Gate {}

impl Hash for Gate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.owner().hash(state);
        self.name().hash(state);
        self.pos().hash(state);
        self.size().hash(state);
    }
}

impl IntoGate for GateRef {
    fn into_gate(self) -> GateRef {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::gate::IntoModuleGate;
    use crate::module::ModuleContext;

    use super::*;

    #[test]
    fn fmt() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let gate = Gate::new(&owner, "port", Some(1));
        assert_eq!(format!("{gate:?}"), "Gate { path: \"root.port[1]\" }");
        assert_eq!(gate.str(), "port[1]");
        assert_eq!(gate.path().as_str(), "root.port[1]");
    }

    #[test]
    fn kind_and_iter() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let gate_a = owner.create_gate("port-a", 0);
        assert_eq!(gate_a.kind(), GateKind::Standalone);

        let gate_b = owner.create_gate("port-b", 0);
        gate_a.clone().connect(gate_b.clone());
        assert_eq!(gate_a.kind(), GateKind::Endpoint);

        let gate_c = owner.create_gate("port-c", 0);
        gate_a.clone().connect(gate_c.clone());
        assert_eq!(gate_a.kind(), GateKind::Transit);

        // Chain chould be c -- a -- b
        assert_eq!(gate_c.next_gate(), Some(gate_a.clone()));
        assert_eq!(gate_c.path_end(), Some(gate_b));

        let mut iter = gate_c.path_iter().unwrap();
        let ca = iter.next().unwrap();
        assert_eq!(ca.prev_hop(), Some(gate_c));
        let ab = iter.next().unwrap();
        assert_eq!(ab.prev_hop(), Some(gate_a));
    }

    #[test]
    fn dedup() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let gate = owner.create_gate("port-a", 0);
        assert_eq!(gate.kind(), GateKind::Standalone);

        let gate_b = owner.create_gate("port-b", 0);
        gate.clone().connect(gate_b.clone());
        assert_eq!(gate.kind(), GateKind::Endpoint);

        gate.clone().connect(gate_b);
        assert_eq!(gate.kind(), GateKind::Endpoint);
    }

    #[test]
    fn connect_into_prev_occupied_slot() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let gate = owner.create_gate("port", 0);

        let gate_a = owner.create_gate("port-a", 0);
        let gate_b = owner.create_gate("port-b", 0);
        let gate_c = owner.create_gate("port-c", 0);

        gate.clone().connect(gate_a.clone());
        gate.clone().connect(gate_b);

        assert_eq!(gate.kind(), GateKind::Transit);
        assert_eq!(gate.connections.read().next_free(), None);
        assert_eq!(
            gate.connections.read().connections[0]
                .as_ref()
                .unwrap()
                .endpoint,
            gate_a
        );

        gate.disconnect(&gate_a);
        assert_eq!(gate.connections.read().next_free(), Some(0));

        gate.clone().connect(gate_c.clone());

        assert_eq!(gate.kind(), GateKind::Transit);
        assert_eq!(gate.connections.read().next_free(), None);
        assert_eq!(
            gate.connections.read().connections[0]
                .as_ref()
                .unwrap()
                .endpoint,
            gate_c
        );
    }

    #[test]
    fn path_iter_at_prev_occupied_slot() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let gate = owner.create_gate("port", 0);

        let gate_a = owner.create_gate("port-a", 0);
        let gate_b = owner.create_gate("port-b", 0);

        gate.clone().connect(gate_a.clone());
        gate.clone().connect(gate_b.clone());
        gate.disconnect(&gate_a);

        assert!(gate.connections.read().connections[0].is_none());
        assert_eq!(gate.kind(), GateKind::Endpoint);
        let mut iter = gate.path_iter().unwrap();
        assert_eq!(iter.next().map(|c| c.endpoint), Some(gate_b))
    }

    #[test]
    fn disconnect() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let gate_a = owner.create_gate("port-a", 0);
        assert_eq!(gate_a.kind(), GateKind::Standalone);

        let gate_b = owner.create_gate("port-b", 0);
        gate_a.clone().connect(gate_b.clone());
        assert_eq!(gate_a.kind(), GateKind::Endpoint);
        assert_eq!(gate_b.kind(), GateKind::Endpoint);

        gate_a.disconnect(&gate_b);
        assert_eq!(gate_a.kind(), GateKind::Standalone);
        assert_eq!(gate_b.kind(), GateKind::Standalone);
    }

    #[test]
    fn disconnect_all() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let a = owner.create_singular_gate("a");
        let b = owner.create_singular_gate("b");
        let c = owner.create_singular_gate("c");

        a.clone().connect(b.clone());
        a.clone().connect(c.clone());

        assert_eq!(a.kind(), GateKind::Transit);
        assert_eq!(b.kind(), GateKind::Endpoint);
        assert_eq!(c.kind(), GateKind::Endpoint);

        a.disconnect_all();

        assert_eq!(a.kind(), GateKind::Standalone);
        assert_eq!(b.kind(), GateKind::Standalone);
        assert_eq!(c.kind(), GateKind::Standalone);
    }

    #[test]
    fn cluster_members() {
        let owner = ModuleContext::new_root("root".into(), Weak::new());
        let a0 = owner.create_gate("a", 0);
        let a1 = owner.create_gate("a", 1);
        let a2 = owner.create_gate("a", 2);

        assert_eq!(a0.cluster(), a1.cluster());
        assert_eq!(a1.cluster(), a2.cluster());
    }

    #[test]
    fn into_gate() {
        let ctx = ModuleContext::new_root("root".into(), Weak::new());
        let gate_a = ctx.create_gate("port-a", 0);

        assert_eq!(gate_a.as_gate(&ctx.ctx), Some(gate_a.clone()));
        assert_eq!(
            Arc::downgrade(&gate_a).as_gate(&ctx.ctx),
            Some(gate_a.clone())
        );
    }
}
