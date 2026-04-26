use crate::prelude::{Channel, ChannelRef};

/// A trait that descibes a channel provider, which will provide channel implementations for a duplex connection.
///
/// By default, each simplex-stream will be managed by a separate handler.
pub trait IntoDuplexChannel {
    /// Splits the channel provider into two channel handlers.
    ///
    /// If the connection should be full-duplex, both handlers should be different.
    /// If the connection should share a single domain, a shared handler should be returned.
    fn into_duplex(self) -> (ChannelRef, ChannelRef);
}

impl<T: Channel + Clone> IntoDuplexChannel for T {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        let bwk = ChannelRef::from(self.clone());
        let fwd = ChannelRef::from(self);
        (fwd, bwk)
    }
}

impl<T: Channel, V: Channel> IntoDuplexChannel for (T, V) {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        (ChannelRef::from(self.0), ChannelRef::from(self.1))
    }
}

impl IntoDuplexChannel for ChannelRef {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        (self.clone(), self)
    }
}

impl IntoDuplexChannel for (ChannelRef, ChannelRef) {
    fn into_duplex(self) -> (ChannelRef, ChannelRef) {
        self
    }
}
