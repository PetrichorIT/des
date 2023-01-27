use std::{
    any::{Any, TypeId},
    fmt,
};

use super::{Plugin, PluginPanicPolicy, PluginStatus};

pub(crate) struct PluginRegistry {
    inner: Vec<PluginEntry>,  // a ordered list of all plugins that are active.
    inject: Vec<PluginEntry>, // plugins to be injected at the next upstream
    pos: Vec<usize>,          // a list of ptrs to the iterators (last() is the current)

    dirty: bool,
    up: bool,
    id: usize,
}

pub(crate) struct PluginEntry {
    pub(super) id: usize,       // a module-specific unqiue identifier
    pub(super) gen: usize,      // the number of restarts the plugin has performed
    pub(super) priority: usize, // 2 bits reserved, public API at least 0b****10

    pub(super) typ: TypeId,
    pub(super) core: Option<Box<dyn Plugin>>,
    pub(super) state: PluginState,

    pub(super) policy: PluginPanicPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum PluginState {
    /// Plugin is not active, but alive, thus self.plugin contains a value.
    Idle = 0,

    /// The plugin is currently being executed. This is only for debug purposes.
    Running = 1,

    /// Plugin is not acitve, but alive, thus self.plugin contains a value.
    /// However it could be the case that the plugin should currently be active
    /// but is not. thus consider this plugin deactived if this state persists
    /// on the downstream path.
    JustCreated = 2,

    /// To be deleted next turn
    PendingRemoval = 3,

    /// Plugin in not active, because its dead, thus self.plugin is empty.
    Paniced = 4,
}

impl PluginRegistry {
    pub(crate) fn new() -> Self {
        Self {
            inner: Vec::new(),
            inject: Vec::new(),
            pos: vec![0],
            dirty: false,
            up: true,
            id: 0,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &PluginEntry> {
        self.inner.iter()
    }

    pub(crate) fn pos(&self) -> usize {
        // SAFTEY:
        // The contract states that at leat one iterator must exist
        unsafe { *self.pos.last().unwrap_unchecked() }
    }

    pub(crate) fn pos_mut(&mut self) -> &mut usize {
        // SAFTEY:
        // The contract states that at leat one iterator must exist
        unsafe { self.pos.last_mut().unwrap_unchecked() }
    }

    pub(crate) fn add(&mut self, mut entry: PluginEntry) -> usize {
        let id = self.id;
        self.id = self.id.wrapping_add(1);
        entry.id = id;

        self.inject.push(entry);
        id
    }

    pub(crate) fn remove(&mut self, id: usize) {
        let i = self.inner.iter().enumerate().find(|(_, p)| p.id == id);
        let Some((i, _)) = i else {
            log::error!("Could not remove plugin for handle '{}: may be removed due to panic policy", id);
            return;
        };

        self.dirty = true;
        self.inner[i].state = PluginState::PendingRemoval;
    }

    pub(crate) fn status(&self, id: usize) -> PluginStatus {
        self.inner.iter().find(|p| p.id == id).map_or_else(
            || {
                self.inject
                    .iter()
                    .find(|p| p.id == id)
                    .map_or(PluginStatus::Gone, |_| PluginStatus::StartingUp)
            },
            PluginStatus::from_entry,
        )
    }

    pub(crate) fn clear(&mut self) {
        self.inner.clear();
        self.inject.clear();
        self.pos.clear();
        self.pos.push(0);
        self.dirty = false;
    }

    pub(crate) fn being_upstream(&mut self, capture: bool) {
        self.up = true;
        self.pos.truncate(1);
        self.pos[0] = 0;

        if !capture {
            // Removes values from the removal queue
            if self.dirty {
                self.inner
                    .retain(|entry| entry.state != PluginState::PendingRemoval);
                self.dirty = false;
            }

            // Add values from inject queue to inner
            for mut entry in self.inject.drain(..) {
                entry.state = PluginState::Idle;
                match self.inner.binary_search(&entry) {
                    Ok(i) | Err(i) => self.inner.insert(i, entry),
                }
            }
        }
    }

    pub(crate) fn next_upstream(&mut self) -> Option<Box<dyn Plugin>> {
        loop {
            let pos = self.pos[0];
            if pos < self.inner.len() {
                if self.inner[pos].activate() {
                    // Real ptr bump
                    self.pos[0] += 1;
                    break self.inner[pos].take();
                }
                self.pos[0] += 1;
            } else {
                break None;
            }
        }
    }

    pub(crate) fn put_back_upstream(&mut self, plugin: Box<dyn Plugin>) {
        let pos = self.pos[0];
        self.inner[pos - 1].core = Some(plugin);
    }

    pub(crate) fn paniced_upstream(&mut self, payload: Box<dyn Any + Send>) {
        let pos = self.pos[0];
        self.inner[pos - 1].state = PluginState::Paniced;
        let policy = self.inner[pos - 1].policy.clone();
        policy.activate(&mut self.inner[pos - 1], payload);
    }

    pub(crate) fn begin_main_downstream(&mut self) {
        self.up = false;
        self.pos.truncate(1);
        self.pos[0] = self.inner.len();
    }

    pub(crate) fn begin_sub_downstream(&mut self, pos: Option<usize>) {
        self.pos.push(pos.unwrap_or(self.pos()));
    }

    pub(crate) fn close_sub_downstream(&mut self) {
        self.pos.pop();
        assert!(!self.pos.is_empty());
    }

    pub(crate) fn next_downstream(&mut self) -> Option<Box<dyn Plugin>> {
        // pos points to a value of the last worked module, maybe invalid index.
        while self.pos() > 0 {
            *self.pos_mut() -= 1;
            if self.inner[self.pos()].is_active() && self.inner[self.pos()].core.is_some() {
                let pos = self.pos();
                return self.inner[pos].take();
            }
        }
        None
    }

    pub(crate) fn put_back_downstream(&mut self, plugin: Box<dyn Plugin>, deactivate: bool) {
        let pos = self.pos();
        self.inner[pos].core = Some(plugin);
        if deactivate && self.inner[pos].state != PluginState::PendingRemoval {
            self.inner[pos].state = PluginState::Idle;
        }
    }

    pub(crate) fn paniced_downstream(&mut self, payload: Box<dyn Any + Send>) {
        let pos = self.pos();
        self.inner[pos].state = PluginState::Paniced;
        let policy = self.inner[pos].policy.clone();
        policy.activate(&mut self.inner[pos], payload);
    }
}

impl PluginEntry {
    #[inline]
    pub(self) fn activate(&mut self) -> bool {
        // let active = matches!(self.state, PluginState::Idle | PluginState::Running);
        let active = self.state as u8 <= 1;
        if active {
            self.state = PluginState::Running;
        }
        active
    }

    #[inline]
    pub(self) fn is_active(&self) -> bool {
        matches!(self.state, PluginState::Running)
    }

    #[inline]
    pub(self) fn take(&mut self) -> Option<Box<dyn Plugin>> {
        self.core.take()
    }
}

impl PartialEq for PluginEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for PluginEntry {}

impl PartialOrd for PluginEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PluginEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl fmt::Debug for PluginEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginEntry")
            .field("id", &self.id)
            .field("priority", &self.priority)
            .field("core", &self.core.is_some())
            .field("state", &self.state)
            .field("policy", &self.policy)
            .finish()
    }
}

// SAFTEY:
// Since plugin entries are stored in a cross thread context
// they must implement this traits. However plugins are not executed
// in a async context, so this does not really matter.
unsafe impl Send for PluginEntry {}
unsafe impl Sync for PluginEntry {}
