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
    handle: Weak<TimerSlot>,
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
        TIME_CTX.with(|ctx| f(&mut ctx.borrow_mut().as_mut().unwrap()))
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
                }
            }
            Err(insert_at) => {
                pending.insert(
                    insert_at,
                    Arc::new(TimerSlot {
                        time,
                        entrys: RefCell::new(vec![entry]),

                        queue: self.clone(),
                    }),
                );
                TimerSlotEntryHandle {
                    id,
                    handle: Arc::downgrade(&pending[insert_at]),
                }
            }
        }
    }

    pub(super) fn next(&self) -> Option<SimTime> {
        self.pending.borrow().front().map(|s| s.time)
    }

    pub(crate) fn bump(&self) -> Vec<TimerSlot> {
        *self.cur.borrow_mut() = SimTime::now();
        let front = match self.pending.borrow().front() {
            Some(v) => v.time,
            None => return Vec::new(),
        };
        if front <= SimTime::now() {
            let mut buffer = Vec::new();
            while let Some(Ok(v)) = self.pending.borrow_mut().pop_front().map(Arc::try_unwrap) {
                buffer.push(v)
            }
            buffer
        } else {
            Vec::new()
        }
    }
}

impl TimerSlot {
    fn add(&self, entry: TimerSlotEntry) {
        self.entrys.borrow_mut().push(entry)
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
            .for_each(|entry| entry.waker.wake())
    }
}

impl TimerSlotEntryHandle {
    pub(super) fn reset(self, new_deadline: SimTime) -> Option<TimerSlotEntryHandle> {
        let handle = self.handle.upgrade()?;
        let entry = handle.remove(self.id)?;
        Some(handle.queue.add(entry, new_deadline))
    }
}

unsafe impl Send for TimerSlotEntry {}
unsafe impl Send for TimerSlotEntryHandle {}
