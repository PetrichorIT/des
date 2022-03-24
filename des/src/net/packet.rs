use std::mem::size_of;

use crate::core::interning::*;
use crate::core::*;
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

///
/// A application-addressed header in a network, similar to TCP/UDP.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
pub struct PacketHeader {
    pub source_node: NodeAddress,
    pub source_port: PortAddress,

    pub dest_node: NodeAddress,
    pub dest_port: PortAddress,

    // should be u8 but test case requies >u16
    pub ttl: usize,
    // should be u8 but test case requies >u16
    pub hop_count: usize,

    pub tos: u8,
    pub protocol: u8,

    pub(crate) seq_no: isize,

    pub(crate) pkt_bit_len: usize,
    pub pkt_byte_len: u16,
}

impl MessageBody for PacketHeader {
    fn bit_len(&self) -> usize {
        size_of::<NodeAddress>() * 16 + size_of::<PortAddress>() * 16 + 48
    }

    fn byte_len(&self) -> usize {
        size_of::<NodeAddress>() * 2 + size_of::<PortAddress>() * 2 + 6
    }
}

///
/// A application-addressed message in a network, similar to TCP/UDP.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[allow(unused)]
#[derive(Debug)]
pub struct Packet {
    header: PacketHeader,
    content: InternedValue<'static>,
}

impl Packet {
    #[deprecated(since = "0.2.0", note = "PacketIDs are no longer supported")]
    pub fn id(&self) -> ! {
        unimplemented!("PacketIDs are no longer supported")
    }

    pub fn header(&self) -> &PacketHeader {
        &self.header
    }

    pub fn set_source_node(&mut self, node: NodeAddress) {
        self.header.source_node = node
    }

    pub fn set_source_port(&mut self, port: PortAddress) {
        self.header.source_port = port
    }

    pub fn set_dest_node(&mut self, node: NodeAddress) {
        self.header.dest_node = node
    }

    pub fn set_dest_port(&mut self, port: PortAddress) {
        self.header.dest_port = port
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

    pub fn set_seq_no(&mut self, seq_no: isize) {
        self.header.seq_no = seq_no
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
        let byte_len = content.byte_len() as u16;

        let interned = unsafe { (*RTC.get()).as_ref().unwrap().interner.intern(content) };

        Self {
            header: PacketHeader {
                source_node: src.0,
                source_port: src.1,

                dest_node: target.0,
                dest_port: target.1,

                ttl: 0,
                hop_count: 0,

                tos: 0,
                protocol: 0,

                seq_no: -1,

                pkt_bit_len: bit_len,
                pkt_byte_len: byte_len,
            },

            content: interned,
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
        self.header.pkt_bit_len as usize + self.header.bit_len()
    }

    fn byte_len(&self) -> usize {
        self.header.pkt_byte_len as usize + self.header.byte_len()
    }
}
