use super::SimTime;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::{Arc, Weak};
use std::task::Waker;

thread_local! {
    static TIME_CTX: RefCell<Option<Driver>> = const { RefCell::new(None)}
}

#[derive(Debug)]
pub(crate) struct Driver {
    pub(crate) next_wakeup: SimTime,
    pub(super) queue: Arc<TimerQueue>,
}

#[derive(Debug)]
pub(super) struct TimerQueue {
    cur: RefCell<SimTime>,
    pending: RefCell<VecDeque<Arc<TimerSlot>>>,
}

#[derive(Debug)]
pub(crate) struct TimerSlot {
    time: SimTime,
    entrys: RefCell<Vec<TimerSlotEntry>>,
    queue: Arc<TimerQueue>,
}

#[derive(Debug)]
pub(super) struct TimerSlotEntry {
    pub(super) waker: Waker,
    pub(super) id: usize,
}

#[derive(Debug)]
pub(super) struct TimerSlotEntryHandle {
    id: usize,
    resolved: bool,
    handle: Weak<TimerSlot>,
}

impl Drop for TimerSlotEntryHandle {
    fn drop(&mut self) {
        if !self.resolved {
            let Some(handle) = self.handle.upgrade() else {
                return;
            };
            let _ = handle.remove(self.id);
        }
    }
}

impl Driver {
    pub(crate) fn new() -> Self {
        Self {
            next_wakeup: SimTime::MAX,
            queue: Arc::new(TimerQueue::new()),
        }
    }

    pub(crate) fn set(self) -> Option<Driver> {
        TIME_CTX.with(|ctx| ctx.borrow_mut().replace(self))
    }

    pub(crate) fn unset() -> Option<Driver> {
        TIME_CTX.with(|ctx| ctx.borrow_mut().take())
    }

    pub(crate) fn next(&self) -> Option<SimTime> {
        self.queue.next()
    }

    pub(crate) fn bump(&self) -> Vec<TimerSlot> {
        self.queue.bump()
    }

    pub(super) fn with_current<R>(f: impl FnOnce(&mut Driver) -> R) -> R {
        TIME_CTX.with(|ctx| {
            f(ctx
                .borrow_mut()
                .as_mut()
                .expect("no IO time driver provided"))
        })
    }
}

impl TimerQueue {
    fn new() -> Self {
        Self {
            cur: RefCell::new(SimTime::ZERO),
            pending: RefCell::new(VecDeque::new()),
        }
    }

    pub(super) fn add(
        self: &Arc<TimerQueue>,
        entry: TimerSlotEntry,
        time: SimTime,
    ) -> TimerSlotEntryHandle {
        let mut pending = self.pending.borrow_mut();
        let id = entry.id;

        match pending.binary_search_by(|slot| slot.time.cmp(&time)) {
            Ok(found) => {
                pending[found].add(entry);
                TimerSlotEntryHandle {
                    id,
                    handle: Arc::downgrade(&pending[found]),
                    resolved: false,
                }
            }
            Err(insert_at) => {
                let slot = TimerSlot::new(time, self.clone());
                slot.add(entry);

                pending.insert(insert_at, Arc::new(slot));
                TimerSlotEntryHandle {
                    id,
                    handle: Arc::downgrade(&pending[insert_at]),
                    resolved: false,
                }
            }
        }
    }

    pub(super) fn next(&self) -> Option<SimTime> {
        self.pending
            .borrow()
            .front()
            .filter(|slot| !slot.entrys.borrow().is_empty())
            .map(|s| s.time)
    }

    pub(crate) fn bump(&self) -> Vec<TimerSlot> {
        let cur = SimTime::now();
        *self.cur.borrow_mut() = cur;
        if self
            .pending
            .borrow()
            .front()
            .map_or(false, |slot| slot.time <= cur)
        {
            let mut buffer = Vec::new();
            let mut pending = self.pending.borrow_mut();
            while pending.front().is_some_and(|slot| slot.time <= cur) {
                let Ok(slot) = Arc::try_unwrap(pending.pop_front().expect("unreachable")) else {
                    continue;
                };
                buffer.push(slot);
            }
            buffer
        } else {
            Vec::new()
        }
    }
}

impl TimerSlot {
    fn new(time: SimTime, queue: Arc<TimerQueue>) -> Self {
        Self {
            time,
            queue,
            entrys: RefCell::new(Vec::with_capacity(2)),
        }
    }

    fn add(&self, entry: TimerSlotEntry) {
        self.entrys.borrow_mut().push(entry);
    }

    fn remove(&self, id: usize) -> Option<TimerSlotEntry> {
        let mut entries = self.entrys.borrow_mut();
        for i in 0..entries.len() {
            if entries[i].id == id {
                return Some(entries.remove(i));
            }
        }

        None
    }

    pub(crate) fn wake_all(self) {
        self.entrys
            .into_inner()
            .into_iter()
            .for_each(|entry| entry.waker.wake());
    }
}

impl TimerSlotEntryHandle {
    pub(super) fn resolve(&mut self) {
        self.resolved = true;
    }

    pub(super) fn reset(self, new_deadline: SimTime) -> Option<TimerSlotEntryHandle> {
        let handle = self.handle.upgrade()?;
        let entry = handle.remove(self.id)?;
        Some(handle.queue.add(entry, new_deadline))
    }
}

unsafe impl Send for TimerSlotEntry {}
unsafe impl Send for TimerSlotEntryHandle {}
