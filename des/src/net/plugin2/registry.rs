use std::any::Any;

use super::{Plugin, PluginEntry, PluginState, PluginStatus};

pub(crate) struct PluginRegistry {
    inner: Vec<PluginEntry>,
    pos: usize,
    up: bool,
    id: usize,
}

impl PluginRegistry {
    pub(crate) fn new() -> Self {
        Self {
            inner: Vec::new(),
            pos: 0,
            up: true,
            id: 0,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &PluginEntry> {
        self.inner.iter()
    }

    pub(crate) fn add(&mut self, mut entry: PluginEntry) -> usize {
        let id = self.id;
        self.id += 1;
        entry.id = id;

        let i = match self.inner.binary_search(&entry) {
            Ok(at) | Err(at) => at,
        };

        if self.up {
            if i >= self.pos {
                // plugin is later in the list, can just be added without chanig anything
                self.inner.insert(i, entry);
            } else {
                // plugin is added before pos, so bump pos, and do not use the plugin
                self.pos += 1;
                self.inner.insert(i, entry);
            }
        } else if i > self.pos {
            self.inner.insert(i, entry);
        } else {
            self.pos += 1;
            self.inner.insert(i, entry);
        }

        id
    }

    pub(crate) fn remove(&mut self, id: usize) {
        let i = self.inner.iter().enumerate().find(|(_, p)| p.id == id);
        let Some((i, _)) = i else {
            log::error!("Could not remove plugin for handle '{}: may be removed due to panic policy", id);
            return;
        };

        if self.up {
            if i >= self.pos {
                self.inner.remove(i);
            } else {
                assert_ne!(i, self.pos - 1, "Cannot remove yourself now");
                self.pos -= 1;
                self.inner.remove(i);
            }
        } else if i > self.pos {
            self.inner.remove(i);
        } else {
            self.pos -= 1;
            self.inner.remove(i);
        }
    }

    pub(crate) fn status(&self, id: usize) -> PluginStatus {
        self.inner
            .iter()
            .find(|p| p.id == id)
            .map(PluginStatus::from_entry)
            .expect("Failed to fetch plugin")
    }

    pub(crate) fn being_upstream(&mut self) {
        self.up = true;
    }

    pub(crate) fn next_upstream(&mut self) -> Option<Box<dyn Plugin>> {
        assert!(self.up);
        loop {
            if self.pos < self.inner.len() {
                if self.inner[self.pos].activate() {
                    // Real ptr bump
                    self.pos += 1;
                    break self.inner[self.pos - 1].take();
                }
                self.pos += 1;
            } else {
                break None;
            }
        }
    }

    pub(crate) fn put_back_upstream(&mut self, plugin: Box<dyn Plugin>) {
        assert!(self.up);
        self.inner[self.pos - 1].plugin = Some(plugin);
        // self.inner[self.pos - 1].state = PluginState::Idle;
    }

    pub(crate) fn paniced_upstream(&mut self, payload: Box<dyn Any + Send>) {
        assert!(self.up);
        self.inner[self.pos - 1].state = PluginState::Paniced;
        let policy = self.inner[self.pos - 1].policy.clone();
        policy.activate(&mut self.inner[self.pos - 1], payload);
    }

    pub(crate) fn begin_downstream(&mut self) {
        self.up = false;
    }

    pub(crate) fn next_downstream(&mut self) -> Option<Box<dyn Plugin>> {
        // pos points to a value of the last worked module, maybe invalid index.
        assert!(!self.up);
        while self.pos > 0 {
            self.pos -= 1;
            dbg!(self.pos);
            if dbg!(self.inner[self.pos].is_active()) {
                return self.inner[self.pos].take();
            }
        }
        None
    }

    pub(crate) fn put_back_downstream(&mut self, plugin: Box<dyn Plugin>) {
        assert!(!self.up);
        self.inner[self.pos].plugin = Some(plugin);
        self.inner[self.pos].state = PluginState::Idle;
    }

    pub(crate) fn paniced_downstream(&mut self, payload: Box<dyn Any + Send>) {
        assert!(!self.up);
        self.inner[self.pos].state = PluginState::Paniced;
        let policy = self.inner[self.pos].policy.clone();
        policy.activate(&mut self.inner[self.pos], payload);
    }
}

impl PluginEntry {
    pub(self) fn activate(&mut self) -> bool {
        let active = matches!(self.state, PluginState::Idle | PluginState::JustCreated);
        if active {
            self.state = PluginState::Running;
        }
        active
    }

    pub(self) fn is_active(&self) -> bool {
        matches!(self.state, PluginState::Running)
    }

    pub(self) fn take(&mut self) -> Option<Box<dyn Plugin>> {
        self.plugin.take()
    }
}
