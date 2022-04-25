use std::any::Any;
use std::rc::Rc;

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
#[derive(Debug, Clone)]
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
    pub last_node: NodeAddress,
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
            packet_length,

            // # TCP header
            src_port: src.1,
            dest_port: dest.1,

            ..Default::default()
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

#[cfg(feature = "net-ipv6")]
impl Default for PacketHeader {
    fn default() -> Self {
        Self {
            // # IPv4 header
            src_node: 0,
            dest_node: 0,

            version: 4,
            traffic_class: 0,
            flow_label: 0,

            packet_length: 0,
            next_header: 0,
            ttl: u8::MAX,

            // # TCP header
            src_port: 0,
            dest_port: 0,

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
            last_node: 0,
        }
    }
}

///
/// A application-addressed header in a network, similar to TCP/UDP.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone)]
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
    pub last_node: NodeAddress,
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
            packet_length,

            // # TCP header
            src_port: src.1,
            dest_port: dest.1,

            ..Default::default()
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

#[cfg(not(feature = "net-ipv6"))]
impl Default for PacketHeader {
    fn default() -> Self {
        Self {
            // # IPv4 header
            src_node: 0,
            dest_node: 0,

            tos: 0,
            packet_length: 0,

            ttl: u8::MAX,

            // # TCP header
            src_port: 0,
            dest_port: 0,
            seq_no: 0,
            ack_no: 0,
            window_size: 0,
            hop_count: 0,
            last_node: 0,
        }
    }
}

///
/// A application-addressed message in a network, similar to TCP/UDP.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Packet {
    pub(crate) header: PacketHeader,
    pub(crate) message_meta: Option<MessageMetadata>,
    pub(crate) content: Option<Rc<dyn Any>>,
}

impl Packet {
    #[deprecated(since = "0.2.0", note = "PacketIDs are no longer supported")]
    pub fn id(&self) -> ! {
        unimplemented!("PacketIDs are no longer supported")
    }

    pub fn meta(&self) -> &MessageMetadata {
        self.message_meta.as_ref().unwrap()
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

    pub fn set_last_node(&mut self, last_node: NodeAddress) {
        self.header.last_node = last_node
    }

    ///
    /// Creates a new instance of self through a builder.
    ///
    pub fn new() -> PacketBuilder {
        PacketBuilder::new()
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

    // pub fn decapsulate<T: 'static + MessageBody>(
    //     self,
    // ) -> (TypedInternedValue<'static, T>, PacketHeader) {
    //     let Self {
    //         content, header, ..
    //     } = self;
    //     (content.unwrap().cast(), header)
    // }

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

    pub fn try_content<T: 'static + MessageBody>(&self) -> Option<&T> {
        Some(self.content.as_ref()?.downcast_ref::<T>()?)
    }

    pub fn content<T: 'static + MessageBody>(&self) -> &T {
        self.try_content().expect("Failed to unwrap")
    }

    pub fn try_content_mut<T: 'static + MessageBody>(&mut self) -> Option<&mut T> {
        let mut_rc = self.content.as_mut()?;
        let mut_any = Rc::get_mut(mut_rc)?;
        Some(mut_any.downcast_mut()?)
    }

    pub fn content_mut<T: 'static + MessageBody>(&mut self) -> &mut T {
        self.try_content_mut().expect("Failed to unwrap")
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

impl From<Packet> for Message {
    fn from(mut pkt: Packet) -> Self {
        // Take the meta away to prevent old metadata after incorrect reconstruction of packet.
        let meta = pkt.message_meta.take().unwrap_or_default();
        Message::new().meta(meta).content(pkt).build()
    }
}

// impl From<TypedInternedValue<'static, Packet>> for Message {
//     fn from(mut pkt: TypedInternedValue<'static, Packet>) -> Self {
//         // Take the meta away to prevent old metadata after incorrect reconstruction of packet.
//         let meta = pkt.message_meta.take().unwrap_or_default();
//         Message::new().meta(meta).content_interned(pkt).build()
//     }
// }

///
/// A intermediary type for constructing packets.
///
pub struct PacketBuilder {
    message_builder: MessageBuilder,
    header: PacketHeader,
    content: Option<(usize, Rc<dyn Any>)>,
}

impl PacketBuilder {
    pub fn new() -> Self {
        Self {
            message_builder: MessageBuilder::new(),
            header: PacketHeader::default(),
            content: None,
        }
    }

    pub fn src(mut self, src_node: NodeAddress, src_port: PortAddress) -> Self {
        self.header.src_node = src_node;
        self.header.src_port = src_port;
        self
    }

    pub fn src_node(mut self, src_node: NodeAddress) -> Self {
        self.header.src_node = src_node;
        self
    }

    pub fn src_port(mut self, src_port: PortAddress) -> Self {
        self.header.src_port = src_port;
        self
    }

    pub fn dest(mut self, dest_node: NodeAddress, dest_port: PortAddress) -> Self {
        self.header.dest_node = dest_node;
        self.header.dest_port = dest_port;
        self
    }

    pub fn dest_node(mut self, dest_node: NodeAddress) -> Self {
        self.header.dest_node = dest_node;
        self
    }

    pub fn dest_port(mut self, dest_port: PortAddress) -> Self {
        self.header.dest_port = dest_port;
        self
    }

    pub fn seq_no(mut self, seq_no: u32) -> Self {
        self.header.seq_no = seq_no;
        self
    }

    pub fn content<T>(mut self, content: T) -> Self
    where
        T: 'static + MessageBody,
    {
        let byte_len = content.byte_len();
        let interned = Rc::new(content);
        self.content = Some((byte_len, interned));
        self
    }

    pub fn content_interned<T>(mut self, content: Rc<T>) -> Self
    where
        T: 'static + MessageBody,
    {
        self.content = Some((content.byte_len(), content));
        self
    }

    // MESSAGE BUILDER EXT

    pub fn id(mut self, id: MessageId) -> Self {
        self.message_builder = self.message_builder.id(id);
        self
    }

    pub fn kind(mut self, kind: MessageKind) -> Self {
        self.message_builder = self.message_builder.kind(kind);
        self
    }

    pub fn timestamp(mut self, timestamp: SimTime) -> Self {
        self.message_builder = self.message_builder.timestamp(timestamp);
        self
    }

    pub fn receiver_module_id(mut self, receiver_module_id: ModuleId) -> Self {
        self.message_builder = self.message_builder.receiver_module_id(receiver_module_id);
        self
    }

    pub fn sender_module_id(mut self, sender_module_id: ModuleId) -> Self {
        self.message_builder = self.message_builder.sender_module_id(sender_module_id);
        self
    }

    pub fn last_gate(mut self, last_gate: GateRef) -> Self {
        self.message_builder = self.message_builder.last_gate(last_gate);
        self
    }

    pub fn creation_time(mut self, creation_time: SimTime) -> Self {
        self.message_builder = self.message_builder.creation_time(creation_time);
        self
    }

    pub fn send_time(mut self, send_time: SimTime) -> Self {
        self.message_builder = self.message_builder.send_time(send_time);
        self
    }

    // END

    pub fn build(self) -> Packet {
        // Packet { header: PacketHeader::new(src, dest, packet_length), content: () }}
        let PacketBuilder {
            message_builder,
            mut header,
            content,
        } = self;

        let (byte_len, content) = match content {
            Some((byte_len, content)) => (byte_len, Some(content)),
            None => (0, None),
        };

        header.packet_length = byte_len as u16;

        let msg = message_builder.build();
        assert!(msg.content.is_none());

        let meta = msg.meta;

        Packet {
            message_meta: Some(meta),
            header,
            content,
        }
    }
}
