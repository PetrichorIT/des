#![allow(dead_code)]

use crate::net::{gate::GateRef, module::ModuleId};
use crate::time::SimTime;

use std::fmt::{Debug, Display};
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
    pub id: MessageId,     // Custom
    pub kind: MessageKind, // Ethertype
    pub creation_time: SimTime,
    pub send_time: SimTime,

    pub sender_module_id: ModuleId,   // MAC src
    pub receiver_module_id: ModuleId, // MAC dest
    pub last_gate: Option<GateRef>,   // Path info

    pub src: [u8; 6],
    pub dest: [u8; 6],

    // The packet length in bytes.
    pub length: u32,
}

// # DUP
impl MessageHeader {
    pub(super) fn dup(&self) -> Self {
        Self {
            id: self.id,
            kind: self.kind,
            creation_time: SimTime::now(),
            send_time: SimTime::MIN,

            sender_module_id: self.sender_module_id,
            receiver_module_id: self.receiver_module_id,
            last_gate: self.last_gate.as_ref().map(Arc::clone),

            src: self.src,
            dest: self.dest,

            length: self.length,
        }
    }
}

impl Default for MessageHeader {
    fn default() -> Self {
        Self {
            id: 0,
            kind: 0,
            creation_time: SimTime::now(),
            send_time: SimTime::MIN,

            sender_module_id: ModuleId::NULL,
            receiver_module_id: ModuleId::NULL,
            last_gate: None,

            src: [0; 6],
            dest: [0; 6],

            length: 0,
        }
    }
}

impl MessageBody for MessageHeader {
    fn byte_len(&self) -> usize {
        64 // TODO  compute correct header size
    }
}

/// The internal typ of the message set by the des not the user.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MessageType {
    /// A user defined message.
    #[default]
    UserDefined,
    /// A custom internal message. Those should never appear in 'handle_message'.
    Internal,
}

impl Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserDefined => write!(f, "UserDefined"),
            Self::Internal => write!(f, "Internal"),
        }
    }
}
