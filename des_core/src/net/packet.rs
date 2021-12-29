use std::{any::TypeId, mem::size_of};

use des_macros::GlobalUID;

use super::MessageBody;

/// A address of a node in a IPv6 network.
#[cfg(feature = "netipv6")]
pub type NodeAddress = u128;

/// The broadcast address in a IPv6 network.
#[cfg(feature = "netipv6")]
pub const NODE_ADDR_BROADCAST: NodeAddress = u128::MAX;

/// The loopback address in a IPv6 network.
#[cfg(feature = "netipv6")]
pub const NODE_ADDR_LOOPBACK: NodeAddress = 0xfe80;

/// A address of a node in a IPv4 network.
#[cfg(not(feature = "netipv6"))]
pub type NodeAddress = u32;

/// The broadcast address in a IPv4 network.
#[cfg(not(feature = "netipv6"))]
pub const NODE_ADDR_BROADCAST: NodeAddress = u32::MAX;

/// The loopback address in a IPv4 network.
#[cfg(not(feature = "netipv6"))]
pub const NODE_ADDR_LOOPBACK: NodeAddress = 0x7f_00_00_01;

/// A node-local address of an application.
pub type PortAddress = u16;

#[derive(GlobalUID)]
#[repr(transparent)]
pub struct PacketId(u32);

///
/// A application-addressed message in a network, similar to TCP/UDP.
///
#[allow(unused)]
pub struct Packet {
    id: PacketId,

    source_node: NodeAddress,
    source_port: PortAddress,

    target_node: NodeAddress,
    target_port: PortAddress,

    ttl: usize,
    hop_count: usize,

    content: *mut (),
    content_ty_id: TypeId,
    content_bit_len: usize,
    content_byte_len: usize,
}

impl Packet {
    /// The unqiue identifer of the given packet.
    #[inline(always)]
    pub fn id(&self) -> PacketId {
        self.id
    }

    /// The source address of the packet.
    #[inline(always)]
    pub fn source_addr(&self) -> NodeAddress {
        self.source_node
    }

    /// The source port of the packet.
    #[inline(always)]
    pub fn source_port(&self) -> PortAddress {
        self.source_port
    }

    /// The requested targets address.
    #[inline(always)]
    pub fn target_addr(&self) -> NodeAddress {
        self.target_node
    }

    /// The requested targets port.
    #[inline(always)]
    pub fn target_port(&self) -> PortAddress {
        self.target_port
    }

    /// The number of hops the message had before.
    pub fn hop_count(&self) -> usize {
        self.hop_count
    }

    /// Sets the hop counter.
    pub fn set_hop_count(&mut self, hop_count: usize) {
        self.hop_count = hop_count
    }

    /// The time to live of a message.
    pub fn ttl(&self) -> usize {
        self.ttl
    }

    /// Sets the TTL.
    pub fn set_ttl(&mut self, ttl: usize) {
        self.ttl = ttl
    }

    ///
    /// Creates a new message with the given metadata and
    /// a content of type T.
    ///
    /// # Guarntees
    ///
    /// The value of type T will be moved into a box which is then
    /// transmuted into a raw ptr. The allocated memory of T will only
    /// be dropped if the message is extracted.
    ///
    pub fn new<T>(
        src: (NodeAddress, PortAddress),
        target: (NodeAddress, PortAddress),
        content: T,
    ) -> Self
    where
        T: 'static + MessageBody,
    {
        let bit_len = content.bit_len();
        let byte_len = content.byte_len();

        let boxed = Box::new(content);
        let ptr = Box::into_raw(boxed) as *mut ();

        Self {
            id: PacketId::gen(),

            source_node: src.0,
            source_port: src.1,

            target_node: target.0,
            target_port: target.1,

            ttl: 0,
            hop_count: 0,

            content: ptr,
            content_ty_id: TypeId::of::<T>(),
            content_bit_len: bit_len,
            content_byte_len: byte_len,
        }
    }

    ///
    /// Consumes the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// # Safty
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    ///
    pub fn extract_content<T: 'static + MessageBody>(self) -> Box<T> {
        assert_eq!(
            self.content_ty_id,
            TypeId::of::<T>(),
            "Extracted content must have the same type T the packet was build with."
        );

        let ptr = self.content as *mut T;
        // SAFTY:
        // Note that this function is incredbilly unsafe but nessecary
        // due to the constrains of the user-defined content.

        unsafe { Box::from_raw(ptr) }
    }

    pub unsafe fn extract_content_ref<T: 'static + MessageBody>(&self) -> &T {
        assert_eq!(
            self.content_ty_id,
            TypeId::of::<T>(),
            "Extracted content must have the same type T the packet was build with."
        );

        let ptr = self.content as *const T;
        let reference: &T = &*ptr;
        reference
    }
}

impl MessageBody for Packet {
    fn bit_len(&self) -> usize {
        self.content_bit_len + 16 * size_of::<NodeAddress>() + 16 * size_of::<PortAddress>()
    }

    fn byte_len(&self) -> usize {
        self.content_byte_len + 2 * size_of::<NodeAddress>() + 2 * size_of::<PortAddress>()
    }
}
