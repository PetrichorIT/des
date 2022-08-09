use std::any::Any;
use std::fmt::Debug;

use crate::net::{
    GateRef, Message, MessageBody, MessageBuilder, MessageId, MessageKind, MessageMetadata,
    ModuleId,
};
use crate::time::SimTime;

cfg_net_v6! {
    /// A address of a node in a IPv6 network.
    pub type NodeAddress = u128;

    /// The broadcast address in a IPv6 network.
    pub const NODE_ADDR_BROADCAST: NodeAddress = u128::MAX;

    /// The loopback address in a IPv6 network.
    pub const NODE_ADDR_LOOPBACK: NodeAddress = 0xfe80;

    ///
    /// A application-addressed header in a network, similar to TCP/UDP.
    #[derive(Debug, Clone)]
    #[allow(missing_docs)]
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

    impl PacketHeader {
        ///
        /// Creates a new instance of `Self`.
        ///
        #[must_use]
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

    impl MessageBody for PacketHeader {
        fn bit_len(&self) -> usize {
            480 + 128
        }

        fn byte_len(&self) -> usize {
            60 + 20
        }
    }

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
}

cfg_net_v4! {
    /// A address of a node in a IPv4 network.
    pub type NodeAddress = u32;

    /// The broadcast address in a IPv4 network.
    pub const NODE_ADDR_BROADCAST: NodeAddress = u32::MAX;

    /// The loopback address in a IPv4 network.
    pub const NODE_ADDR_LOOPBACK: NodeAddress = 0x7f_00_00_01;

    ///
    /// A application-addressed header in a network, similar to TCP/UDP.
    ///
    #[derive(Debug, Clone)]
    #[allow(missing_docs)]
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

    impl PacketHeader {
        ///
        /// Creates a new instance of `Self`.
        ///
        #[must_use]
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

    impl MessageBody for PacketHeader {
        fn bit_len(&self) -> usize {
            160 + 128
        }

        fn byte_len(&self) -> usize {
            20 + 20
        }
    }

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
}

/// A node-local address of an application.
pub type PortAddress = u16;

///
/// A application-addressed message in a network, similar to TCP/UDP.
///
#[allow(unused)]
#[derive(Debug)]
pub struct Packet {
    pub(crate) header: PacketHeader,
    pub(crate) message_meta: Option<MessageMetadata>,
    pub(crate) content: Option<Box<dyn Any>>,
}

impl Packet {
    ///
    /// Returns the attached [`MessageMetadata`] of the attached [Message].
    ///
    /// # Panics
    ///
    /// Panics if no message metadata was attached.
    ///
    #[must_use]
    pub fn meta(&self) -> &MessageMetadata {
        self.message_meta.as_ref().unwrap()
    }

    ///
    /// Returns the header as a readonly ref.
    ///
    #[must_use]
    pub fn header(&self) -> &PacketHeader {
        &self.header
    }

    ///
    /// Sets the source node of the packet.
    ///
    pub fn set_source_node(&mut self, node: NodeAddress) {
        self.header.src_node = node;
    }

    ///
    /// Sets the source port of the packet.
    ///
    pub fn set_source_port(&mut self, port: PortAddress) {
        self.header.src_port = port;
    }

    ///
    /// Sets the destintation node of the packet.
    ///
    pub fn set_dest_node(&mut self, node: NodeAddress) {
        self.header.dest_node = node;
    }

    ///
    /// Sets the destintation port of the packet.
    ///
    pub fn set_dest_port(&mut self, port: PortAddress) {
        self.header.dest_port = port;
    }

    ///
    /// Sets the packets time to live.
    ///
    pub fn set_ttl(&mut self, ttl: u8) {
        self.header.ttl = ttl;
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
        self.header.seq_no = seq_no;
    }

    ///
    /// Set s the last node to the packet header.
    ///
    pub fn set_last_node(&mut self, last_node: NodeAddress) {
        self.header.last_node = last_node;
    }

    ///
    /// Creates a new instance of self through a builder.
    ///
    #[allow(clippy::new_ret_no_self)]
    #[must_use]
    pub fn new() -> PacketBuilder {
        PacketBuilder::new()
    }

    ///
    /// Trys to return the content by reference casted to the given type T.
    /// Returns [None] if the no content exists or the content is not of type T.
    ///
    #[must_use]
    pub fn try_content<T: 'static + MessageBody>(&self) -> Option<&T> {
        Some(self.content.as_ref()?.downcast_ref::<T>())?
    }

    ///
    /// Trys to return the content by reference casted to the given type T.
    /// Panics if the no content exists or the content is not of type T.
    ///
    #[must_use]
    pub fn content<T: 'static + MessageBody>(&self) -> &T {
        self.try_content().expect("Failed to unwrap")
    }

    ///
    /// Trys to return the content by mutable ref casted to the given type T.
    /// Returns [None] if the no content exists or the content is not of type T.
    ///
    pub fn try_content_mut<T: 'static + MessageBody>(&mut self) -> Option<&mut T> {
        let mut_box = self.content.as_mut()?;
        Some(mut_box.downcast_mut())?
    }

    ///
    /// Trys to return the content by mutable ref casted to the given type T.
    /// Panics if the no content exists or the content is not of type T.
    ///
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

// SAFTY:
// See [Message] contract its the same
unsafe impl Send for Packet {}

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
    content: Option<(usize, Box<dyn Any>)>,
}

impl PacketBuilder {
    ///
    /// Creates a new [`PacketBuilder`].
    ///
    #[must_use]
    pub fn new() -> Self {
        Self {
            message_builder: MessageBuilder::new(),
            header: PacketHeader::default(),
            content: None,
        }
    }

    /// Sets the field `src_node` and `src_port`.
    #[must_use]
    pub fn src(mut self, src_node: NodeAddress, src_port: PortAddress) -> Self {
        self.header.src_node = src_node;
        self.header.src_port = src_port;
        self
    }

    /// Sets the field `src_node`.
    #[must_use]
    pub fn src_node(mut self, src_node: NodeAddress) -> Self {
        self.header.src_node = src_node;
        self
    }

    /// Sets the field `src_port`.
    #[must_use]
    pub fn src_port(mut self, src_port: PortAddress) -> Self {
        self.header.src_port = src_port;
        self
    }

    /// Sets the field `dest_node` and `dest_port`
    #[must_use]
    pub fn dest(mut self, dest_node: NodeAddress, dest_port: PortAddress) -> Self {
        self.header.dest_node = dest_node;
        self.header.dest_port = dest_port;
        self
    }

    /// Sets the field `dest_node`.
    #[must_use]
    pub fn dest_node(mut self, dest_node: NodeAddress) -> Self {
        self.header.dest_node = dest_node;
        self
    }

    /// Sets the field `dest_port`.
    #[must_use]
    pub fn dest_port(mut self, dest_port: PortAddress) -> Self {
        self.header.dest_port = dest_port;
        self
    }

    /// Sets the field `seq_no`.
    #[must_use]
    pub fn seq_no(mut self, seq_no: u32) -> Self {
        self.header.seq_no = seq_no;
        self
    }

    /// Sets the field `content`.
    #[must_use]
    pub fn content<T>(mut self, content: T) -> Self
    where
        T: 'static + MessageBody,
    {
        let byte_len = content.byte_len();
        let interned = Box::new(content);
        self.content = Some((byte_len, interned));
        self
    }

    /// Sets the field `content`.
    #[must_use]
    pub fn content_boxed<T>(mut self, content: Box<T>) -> Self
    where
        T: 'static + MessageBody,
    {
        self.content = Some((content.byte_len(), content));
        self
    }

    // MESSAGE BUILDER EXT

    /// Sets the field `msg.id`.
    #[must_use]
    pub fn id(mut self, id: MessageId) -> Self {
        self.message_builder = self.message_builder.id(id);
        self
    }

    /// Sets the field `msg.kind`.
    #[must_use]
    pub fn kind(mut self, kind: MessageKind) -> Self {
        self.message_builder = self.message_builder.kind(kind);
        self
    }

    /// Sets the field `msg.timestamp`.
    #[must_use]
    pub fn timestamp(mut self, timestamp: SimTime) -> Self {
        self.message_builder = self.message_builder.timestamp(timestamp);
        self
    }

    /// Sets the field `msg.receiver_module_id`.
    #[must_use]
    pub fn receiver_module_id(mut self, receiver_module_id: ModuleId) -> Self {
        self.message_builder = self.message_builder.receiver_module_id(receiver_module_id);
        self
    }

    /// Sets the field `msg.sender_module_id`.
    #[must_use]
    pub fn sender_module_id(mut self, sender_module_id: ModuleId) -> Self {
        self.message_builder = self.message_builder.sender_module_id(sender_module_id);
        self
    }

    /// Sets the field `msg.last_gate`.
    #[must_use]
    pub fn last_gate(mut self, last_gate: GateRef) -> Self {
        self.message_builder = self.message_builder.last_gate(last_gate);
        self
    }

    /// Sets the field `msg.creation_time`.
    #[must_use]
    pub fn creation_time(mut self, creation_time: SimTime) -> Self {
        self.message_builder = self.message_builder.creation_time(creation_time);
        self
    }

    /// Sets the field `msg.send_time`.
    #[must_use]
    pub fn send_time(mut self, send_time: SimTime) -> Self {
        self.message_builder = self.message_builder.send_time(send_time);
        self
    }

    // END

    ///
    /// Builds a [Packet] from the values given in the builder.
    ///
    /// # Panics
    ///
    /// Panics if the contained message metadate points
    /// to content that is not this packet.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
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

impl Default for PacketBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for PacketBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PacketBuilder")
    }
}
