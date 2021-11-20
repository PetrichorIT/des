use std::mem::size_of;

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

///
/// A application-addressed message in a network, similar to TCP/UDP.
///
#[allow(unused)]
pub struct Packet<T: MessageBody> {
    source_node: NodeAddress,
    source_port: PortAddress,

    target_node: NodeAddress,
    target_port: PortAddress,

    content: T,
}

impl<T: MessageBody> MessageBody for Packet<T> {
    fn bit_len(&self) -> usize {
        self.content.bit_len() + 16 * size_of::<NodeAddress>() + 16 * size_of::<PortAddress>()
    }

    fn byte_len(&self) -> usize {
        self.content.byte_len() + 2 * size_of::<NodeAddress>() + 2 * size_of::<PortAddress>()
    }
}
