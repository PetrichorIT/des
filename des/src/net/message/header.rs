#![allow(dead_code)]

use crate::net::{GateRef, ModuleId};
use crate::time::SimTime;

use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use super::MessageBody;

///
/// A ID that defines the meaning of the message in the simulation context.
///
///  * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub type MessageId = u16;

///
/// The type of messages, similar to the TOS field in IP packets.
///
///  * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub type MessageKind = u16;

///
/// The metadata attachted to a message, independent of its contents.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct MessageHeader {
    pub(crate) typ: u8,

    pub id: MessageId,
    pub kind: MessageKind,
    pub creation_time: SimTime,
    pub send_time: SimTime,

    pub sender_module_id: ModuleId,
    pub receiver_module_id: ModuleId,
    pub last_gate: Option<GateRef>,

    pub src_addr: SocketAddr,
    pub dest_addr: SocketAddr,

    pub version: u8,
    pub traffic_class: u8,
    pub flow_label: u32,

    pub next_header: u8,
    pub ttl: u8,
    pub hop_count: usize,

    pub seq_no: u32,
    pub ack_no: u32,
    pub win_size: u16,
    pub flags: MessageHeaderFlags,

    // The packet length in bytes.
    pub length: u32,
}

impl MessageHeader {
    /// Returns the type of the message
    #[must_use]
    pub fn typ(&self) -> MessageType {
        match self.typ {
            0 => MessageType::UserDefined,
            TYP_WAKEUP | TYP_RESTART => MessageType::Internal,
            TYP_TCP_CONNECT | TYP_TCP_CONNECT_TIMEOUT | TYP_TCP_PACKET => MessageType::Tcp,
            TYP_UDP_PACKET => MessageType::Udp,
            _ => unreachable!(),
        }
    }
}

// # DUP
impl MessageHeader {
    pub(super) fn dup(&self) -> Self {
        Self {
            typ: self.typ,

            id: self.id,
            kind: self.kind,
            creation_time: SimTime::now(),
            send_time: SimTime::MAX,

            sender_module_id: self.sender_module_id,
            receiver_module_id: self.receiver_module_id,
            last_gate: self.last_gate.as_ref().map(Arc::clone),

            src_addr: self.src_addr,
            dest_addr: self.dest_addr,

            version: self.version,
            traffic_class: self.traffic_class,
            flow_label: self.flow_label,

            next_header: self.next_header,
            ttl: self.ttl,
            hop_count: self.hop_count,

            seq_no: self.seq_no,
            ack_no: self.ack_no,
            win_size: self.win_size,
            flags: self.flags,

            length: self.length,
        }
    }
}

impl Default for MessageHeader {
    fn default() -> Self {
        Self {
            typ: 0,

            id: 0,
            kind: 0,
            creation_time: SimTime::now(),
            send_time: SimTime::MAX,

            sender_module_id: ModuleId::NULL,
            receiver_module_id: ModuleId::NULL,
            last_gate: None,

            src_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            dest_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),

            version: 4,
            traffic_class: 0,
            flow_label: 0,

            next_header: 0,
            ttl: 64,
            hop_count: 0,

            seq_no: 0,
            ack_no: 0,
            win_size: 0,
            flags: MessageHeaderFlags::default(),

            length: 0,
        }
    }
}

impl MessageBody for MessageHeader {
    fn byte_len(&self) -> usize {
        64 // TODO  compute correct header size
    }
}

/// Flags of a message header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MessageHeaderFlags {
    inner: u8,
}

pub(crate) const TYP_RESTART: u8 = 10;
pub(crate) const TYP_WAKEUP: u8 = 11;
pub(crate) const TYP_IO_TICK: u8 = 12;
pub(crate) const TYP_UDP_PACKET: u8 = 100;
pub(crate) const TYP_TCP_CONNECT: u8 = 101;
pub(crate) const TYP_TCP_CONNECT_TIMEOUT: u8 = 102;
pub(crate) const TYP_TCP_PACKET: u8 = 103;

/// The internal typ of the message set by the des not the user.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MessageType {
    /// A user defined message.
    #[default]
    UserDefined,
    /// A internal TCP message that will be consumed by the IOContext
    /// if possible.
    Tcp,
    /// A internal UDP message that will be consumed by the IOContext
    /// if possible.
    Udp,
    /// A custom internal message. Those should never appear in 'handle_message'.
    Internal,
}
