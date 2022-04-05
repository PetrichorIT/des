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
#[cfg(feature = "net-ipv6")]
pub struct PacketHeader {
    // # Ipv6 Header
    pub src_node: NodeAddress,
    pub dest_node: NodeAddress,

    pub version: u8,
    pub traffic_class: u8,
    pub flow_label: u32,

    pub packet_length: u16,
    pub next_header: u8,
    pub ttl: u8,

    // # TCP header
    pub src_port: PortAddress,
    pub dest_port: PortAddress,

    pub seq_no: u32,
    pub ack_no: u32,
    pub data_offset: u8,

    pub flag_ns: bool,
    pub flag_cwr: bool,
    pub flag_ece: bool,
    pub flag_urg: bool,
    pub flag_ack: bool,
    pub flag_psh: bool,
    pub flag_rst: bool,
    pub flag_syn: bool,
    pub flag_fin: bool,

    pub window_size: u16,
    pub tcp_checksum: u16,
    pub urgent_ptr: u16,

    //# Custom headers
    pub hop_count: usize,
}

#[cfg(feature = "net-ipv6")]
impl PacketHeader {
    ///
    /// Creates a new instance of `Self`.
    ///
    pub fn new(
        src: (NodeAddress, PortAddress),
        dest: (NodeAddress, PortAddress),
        packet_length: u16,
    ) -> Self {
        Self {
            // # IPv4 header
            src_node: src.0,
            dest_node: dest.0,

            version: 4,
            traffic_class: 0,
            flow_label: 0,

            packet_length,
            next_header: 0,
            ttl: u8::MAX,

            // # TCP header
            src_port: src.1,
            dest_port: dest.1,

            seq_no: 0,
            ack_no: 0,
            data_offset: 0,

            flag_ns: false,
            flag_cwr: false,
            flag_ece: false,
            flag_urg: false,
            flag_ack: false,
            flag_psh: false,
            flag_rst: false,
            flag_syn: false,
            flag_fin: false,

            window_size: 0,
            tcp_checksum: 0,
            urgent_ptr: 0,

            hop_count: 0,
        }
    }
}

#[cfg(feature = "net-ipv6")]
impl MessageBody for PacketHeader {
    fn bit_len(&self) -> usize {
        480 + 128
    }

    fn byte_len(&self) -> usize {
        60 + 20
    }
}

///
/// A application-addressed header in a network, similar to TCP/UDP.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
#[cfg(not(feature = "net-ipv6"))]
pub struct PacketHeader {
    // # IPv4 header
    pub src_node: NodeAddress,
    pub dest_node: NodeAddress,
    pub tos: u8,
    pub packet_length: u16,
    pub ttl: u8,

    // # TCP header
    pub src_port: PortAddress,
    pub dest_port: PortAddress,

    pub seq_no: u32,
    pub ack_no: u32,
    pub window_size: u16,

    //# Custom headers
    pub hop_count: usize,
}

#[cfg(not(feature = "net-ipv6"))]
impl PacketHeader {
    ///
    /// Creates a new instance of `Self`.
    ///
    pub fn new(
        src: (NodeAddress, PortAddress),
        dest: (NodeAddress, PortAddress),
        packet_length: u16,
    ) -> Self {
        Self {
            // # IPv4 header
            src_node: src.0,
            dest_node: dest.0,

            tos: 0,
            packet_length,

            ttl: u8::MAX,

            // # TCP header
            src_port: src.1,
            dest_port: dest.1,
            seq_no: 0,
            ack_no: 0,
            window_size: 0,
            hop_count: 0,
        }
    }
}

#[cfg(not(feature = "net-ipv6"))]
impl MessageBody for PacketHeader {
    fn bit_len(&self) -> usize {
        160 + 128
    }

    fn byte_len(&self) -> usize {
        20 + 20
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

    ///
    /// Returns the header as a readonly ref.
    ///
    pub fn header(&self) -> &PacketHeader {
        &self.header
    }

    ///
    /// Sets the source node of the packet.
    ///
    pub fn set_source_node(&mut self, node: NodeAddress) {
        self.header.src_node = node
    }

    ///
    /// Sets the source port of the packet.
    ///
    pub fn set_source_port(&mut self, port: PortAddress) {
        self.header.src_port = port
    }

    ///
    /// Sets the destintation node of the packet.
    ///
    pub fn set_dest_node(&mut self, node: NodeAddress) {
        self.header.dest_node = node
    }

    ///
    /// Sets the destintation port of the packet.
    ///
    pub fn set_dest_port(&mut self, port: PortAddress) {
        self.header.dest_port = port
    }

    ///
    /// Sets the packets time to live.
    ///
    pub fn set_ttl(&mut self, ttl: u8) {
        self.header.ttl = ttl
    }

    ///
    /// Registers a hop in the header, thereby decrementing ttl
    /// while incrementing the hop count.
    ///
    pub fn register_hop(&mut self) {
        self.header.ttl = self.header.ttl.wrapping_sub(1);
        self.header.hop_count += 1;
    }

    ///
    /// Sets the sequence number of the packet.
    ///
    pub fn set_seq_no(&mut self, seq_no: u32) {
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
        dest: (NodeAddress, PortAddress),
        content: T,
    ) -> Self
    where
        T: 'static + MessageBody,
    {
        let byte_len = content.byte_len() as u16;

        let interned = RTC.as_ref().as_ref().unwrap().interner.intern(content);

        Self {
            header: PacketHeader::new(src, dest, byte_len),

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
        (self.header.packet_length as usize * 8) + self.header.bit_len()
    }

    fn byte_len(&self) -> usize {
        self.header.packet_length as usize + self.header.byte_len()
    }
}
