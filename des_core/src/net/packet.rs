use std::mem::size_of;

use crate::core::interning::*;
use crate::core::*;
use crate::create_global_uid;
use crate::net::*;

/// A address of a node in a IPv6 network.
#[cfg(feature = "net-ipv6")]
pub type NodeAddress = u128;

/// The broadcast address in a IPv6 network.
#[cfg(feature = "net-ipv6")]
pub const NODE_ADDR_BROADCAST: NodeAddress = u128::MAX;

/// The loopback address in a IPv6 network.
#[cfg(feature = "net-ipv6")]
pub const NODE_ADDR_LOOPBACK: NodeAddress = 0xfe80;

/// A address of a node in a IPv4 network.
#[cfg(not(feature = "net-ipv6"))]
pub type NodeAddress = u32;

/// The broadcast address in a IPv4 network.
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[cfg(not(feature = "net-ipv6"))]
#[allow(unused)]
pub const NODE_ADDR_BROADCAST: NodeAddress = u32::MAX;

/// The loopback address in a IPv4 network.
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[cfg(not(feature = "net-ipv6"))]
#[allow(unused)]
pub const NODE_ADDR_LOOPBACK: NodeAddress = 0x7f_00_00_01;

/// A node-local address of an application.
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub type PortAddress = u16;

create_global_uid!(
    /// A globalsy unqiue identifer for a packet.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub PacketId(u32) = PACKET_ID;
);

///
/// A application-addressed header in a network, similar to TCP/UDP.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
pub struct PacketHeader {
    pub source_node: NodeAddress,
    pub source_port: PortAddress,

    pub target_node: NodeAddress,
    pub target_port: PortAddress,

    pub ttl: usize,
    pub hop_count: usize,
}

///
/// A application-addressed message in a network, similar to TCP/UDP.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[allow(unused)]
#[derive(Debug)]
pub struct Packet {
    id: PacketId,

    header: PacketHeader,

    content: InternedValue<'static>,

    content_bit_len: usize,
    content_byte_len: usize,
}

impl Packet {
    /// The unqiue identifer of the given packet.
    #[inline(always)]
    pub fn id(&self) -> PacketId {
        self.id
    }

    pub fn header(&self) -> &PacketHeader {
        &self.header
    }

    /// Sets the hop counter.
    #[inline(always)]
    pub fn set_hop_count(&mut self, hop_count: usize) {
        self.header.hop_count = hop_count
    }

    /// Increments the hop counter.
    #[inline(always)]
    pub fn inc_hop_count(&mut self) {
        self.header.hop_count += 1;
    }

    /// Sets the TTL.
    #[inline(always)]
    pub fn set_ttl(&mut self, ttl: usize) {
        self.header.ttl = ttl
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

        let interned = unsafe { (*RTC.get()).as_ref().unwrap().interner.intern(content) };

        Self {
            id: PacketId::gen(),

            header: PacketHeader {
                source_node: src.0,
                source_port: src.1,

                target_node: target.0,
                target_port: target.1,

                ttl: 0,
                hop_count: 0,
            },

            content: interned,

            content_bit_len: bit_len,
            content_byte_len: byte_len,
        }
    }

    ///
    /// Extracts the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// # Safty
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    ///
    pub fn decapsulate<T: 'static + MessageBody>(
        self,
    ) -> (TypedInternedValue<'static, T>, PacketHeader) {
        let Self {
            content, header, ..
        } = self;
        (content.cast(), header)
    }

    ///
    /// Extracts the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// # Safty
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    ///
    pub fn content<T: 'static + MessageBody>(&self) -> TypedInternedValue<'static, T> {
        self.content.clone().cast()
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
