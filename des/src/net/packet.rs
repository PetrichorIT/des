use crate::net::{
    GateRef, Message, MessageBody, MessageBuilder, MessageId, MessageKind, MessageMetadata,
    ModuleId,
};
use crate::time::SimTime;
use crate::util::AnyBox;
use std::fmt::Debug;

cfg_net_std! {
    /// A address of a node in a IPv6 network.
    pub type NodeAddress = std::net::IpAddr;

    /// The broadcast address in a IPv6 network.
    pub const NODE_ADDR_BROADCAST: NodeAddress = std::net::IpAddr::V4(std::net::Ipv4Addr::BROADCAST);

    /// The loopback address in a IPv6 network.
    pub const NODE_ADDR_LOOPBACK: NodeAddress = std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);

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
                src_node: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                dest_node: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),

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
                last_node: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            }
        }
    }
}

cfg_net_default! {
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
    pub(crate) content: Option<AnyBox>,
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
    /// Duplicates a message.
    ///
    /// # Panics
    ///
    /// Panics if the contained value is not of type T.
    ///
    #[must_use]
    pub fn dup<T>(&self) -> Self
    where
        T: 'static + Clone,
    {
        self.try_dup::<T>().expect("Failed to duplicate a message")
    }

    ///
    /// Tries to create a duplicate of the message, assuming its content is of type T.
    ///
    /// - If the messages body is of type T, the body will be cloned as specified by T
    /// and the dup will succeed.
    /// - If the message body is not of type T, this function will return `None`.
    /// - If the message has no body it will succeed independent of T and clone only the
    /// attached metadata.
    ///
    #[must_use]
    pub fn try_dup<T>(&self) -> Option<Self>
    where
        T: 'static + Clone,
    {
        let content: Option<AnyBox> = if let Some(ref content) = self.content {
            Some(content.try_dup::<T>()?)
        } else {
            None
        };

        let message_meta = self.message_meta.as_ref().map(|m| m.dup());
        let header = self.header.clone();

        Some(Self {
            message_meta,
            content,
            header,
        })
    }

    // ' INSERT BEGIN

    ///
    /// Consumes the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    ///
    #[must_use]
    pub fn try_cast<T: 'static + MessageBody + Send>(self) -> Result<(T, PacketHeader), Self> {
        // SAFTY:
        // Since T is 'Send' this is safe within the bounds of Messages safty contract
        unsafe { self.try_cast_unsafe::<T>() }
    }

    ///
    /// Performs a [`try_cast`] unwraping the result.
    ///
    #[must_use]
    pub fn cast<T: 'static + MessageBody + Send>(self) -> (T, PacketHeader) {
        self.try_cast().expect("Could not cast to type T")
    }

    ///
    /// Consumes the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    /// Note that this function allows T to be !Send. Be aware of safty problems arriving
    /// from this.
    ///
    #[must_use]
    pub unsafe fn try_cast_unsafe<T: 'static + MessageBody>(
        self,
    ) -> Result<(T, PacketHeader), Self> {
        let Packet {
            header,
            content,
            message_meta,
        } = self;
        let content = match content.map(|c| c.try_cast_unsafe::<T>()) {
            Some(Ok(c)) => c,
            Some(Err(content)) => {
                return Err(Self {
                    content: Some(content),
                    header,
                    message_meta,
                })
            }
            None => {
                return Err(Self {
                    content: None,
                    header,
                    message_meta,
                })
            }
        };

        Ok((content, header))
    }

    ///
    /// Performs a [`try_cast_unsafe`] unwraping the result.
    ///
    /// # Safety
    ///
    /// See [`try_cast_unsafe`]
    #[must_use]
    pub unsafe fn cast_unsafe<T: 'static + MessageBody>(self) -> (T, PacketHeader) {
        self.try_cast_unsafe().expect("Could not cast to type T")
    }

    // ' INSERT END

    ///
    /// Trys to return the content by reference casted to the given type T.
    /// Returns [None] if the no content exists or the content is not of type T.
    ///
    #[must_use]
    pub fn try_content<T: 'static + MessageBody>(&self) -> Option<&T> {
        Some(self.content.as_ref()?.try_cast_ref::<T>())?
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
        Some(self.content.as_mut()?.try_cast_mut())?
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
    content: Option<(usize, AnyBox)>,
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

    #[allow(unused)]
    pub(crate) fn typ(mut self, typ: u8) -> Self {
        self.message_builder = self.message_builder.typ(typ);
        self
    }

    #[allow(unused)]
    pub(crate) fn meta(mut self, meta: MessageMetadata) -> Self {
        self.message_builder = self.message_builder.meta(meta);
        self
    }

    #[allow(unused)]
    pub(crate) fn header(mut self, header: PacketHeader) -> Self {
        self.header = header;
        self
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

    /// Sets the field `dest_node` and `dest_port`
    #[must_use]
    #[cfg(feature = "std-net")]
    pub fn dest_addr(mut self, dest: std::net::SocketAddr) -> Self {
        self.header.dest_node = dest.ip();
        self.header.dest_port = dest.port();
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
        let interned = AnyBox::new(content);
        self.content = Some((byte_len, interned));
        self
    }

    /// Sets the field `content`.
    #[must_use]
    pub fn content_boxed<T>(mut self, content: Box<T>) -> Self
    where
        T: 'static + MessageBody,
    {
        self.content = Some((content.byte_len(), AnyBox::new(Box::into_inner(content))));
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
