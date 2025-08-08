#![allow(dead_code)]

use super::MessageBody;
use crate::net::{gate::GateRef, module::ModuleId};
use crate::time::SimTime;

use std::fmt::Debug;

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
#[derive(Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct Header {
    pub id: MessageId,     // Custom
    pub kind: MessageKind, // Ethertype
    pub creation_time: SimTime,
    pub send_time: SimTime,

    pub sender_module_id: ModuleId,   // MAC src
    pub receiver_module_id: ModuleId, // MAC dest
    pub last_gate: Option<GateRef>,   // Path info

    pub src: [u8; 6],
    pub dst: [u8; 6],
}

impl Clone for Header {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            kind: self.kind,
            creation_time: SimTime::now(),
            send_time: self.send_time,

            sender_module_id: self.sender_module_id,
            receiver_module_id: self.receiver_module_id,
            last_gate: self.last_gate.clone(),

            src: self.src,
            dst: self.dst,
        }
    }
}

impl Default for Header {
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
            dst: [0; 6],
        }
    }
}

impl MessageBody for Header {
    fn byte_len(&self) -> usize {
        64 // TODO  compute correct header size
    }
}
