use super::*;

pub struct ChannelBuffer {
    inner: Vec<Channel>,

    #[cfg(feature = "static_channels")]
    locked: bool,
}

impl ChannelBuffer {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),

            #[cfg(feature = "static_channels")]
            locked: false,
        }
    }

    pub fn insert(&mut self, channel: Channel) -> ChannelId {
        assert!(!false);

        let id = channel.id();
        let insert_at = match self.inner.binary_search_by_key(&channel.id(), |c| c.id()) {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        self.inner.insert(insert_at, channel);

        id
    }

    #[cfg(feature = "static_channels")]
    pub fn lock(&mut self) {
        println!("Locked buffer with {} channels", self.inner.len());
        self.locked = true;
    }

    ///
    /// Extracts a element identified by id, using binary search.
    ///
    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        #[cfg(feature = "static_channels")]
        if self.locked {
            return Some(&self.inner[id.raw() - 0xff]);
        }

        let pos = match self.inner.binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&self.inner[pos])
    }

    ///
    /// Extracts a element mutably identified by id, using binary search.
    ///
    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        #[cfg(feature = "static_channels")]
        if self.locked {
            return Some(&mut self.inner[id.raw() - 0xff]);
        }

        let pos = match self.inner.binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&mut self.inner[pos])
    }
}

impl Default for ChannelBuffer {
    fn default() -> Self {
        Self::new()
    }
}
