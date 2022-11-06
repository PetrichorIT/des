use std::{fmt::Debug, time::Duration};

#[derive(Clone)]
pub struct CacheOptimizedLinkedList<E> {
    next: Vec<usize>,
    prev: Vec<usize>,
    id: Vec<usize>,
    time: Vec<Duration>,
    body: Vec<Option<E>>,

    head: usize,
    tail: usize,
    len: usize,

    free_list: Vec<usize>,
}

impl<E> CacheOptimizedLinkedList<E> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn new() -> Self {
        Self::with_capacity(8)
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            next: vec![usize::MAX; cap],
            prev: vec![usize::MAX; cap],
            id: vec![0; cap],
            time: vec![Duration::MAX; cap],
            body: std::iter::repeat_with(|| None).take(cap).collect(),

            head: usize::MAX,
            tail: usize::MAX,
            len: 0,
            free_list: (0..cap).collect(),
        }
    }

    pub fn add(&mut self, event: E, time: Duration, id: usize) {
        if self.is_empty() {
            // Assume that all is in its inital state
            let slot = if let Some(slot) = self.free_list.pop() {
                slot
            } else {
                self.grow();
                self.free_list.pop().unwrap()
            };

            self.next[slot] = usize::MAX;
            self.prev[slot] = usize::MAX;
            self.time[slot] = time;
            self.body[slot] = Some(event);
            self.id[slot] = id;

            self.head = slot;
            self.tail = slot;
        } else {
            // println!("{:?}\n{:?}\n{:?}", self.prev, self.next, self.time);
            // println!("{}, {}", self.head, self.tail);
            let mut cur = self.tail; // valid ptr
            while cur != usize::MAX && self.time[cur] > time {
                cur = self.prev[cur]
            }
            // println!("Found insert pos: {}", )
            if cur == usize::MAX {
                let slot = if let Some(slot) = self.free_list.pop() {
                    slot
                } else {
                    self.grow();
                    self.free_list.pop().unwrap()
                };
                self.next[slot] = self.head;
                self.prev[slot] = usize::MAX;
                self.time[slot] = time;
                self.body[slot] = Some(event);
                self.id[slot] = id;

                self.prev[self.head] = slot;
                self.head = slot;
            } else {
                let slot = if let Some(slot) = self.free_list.pop() {
                    slot
                } else {
                    self.grow();
                    self.free_list.pop().unwrap()
                };
                // slot locals
                self.next[slot] = self.next[cur];
                self.prev[slot] = cur;

                self.time[slot] = time;
                self.body[slot] = Some(event);
                self.id[slot] = id;

                // slot links
                self.next[cur] = slot;
                if self.next[slot] != usize::MAX {
                    self.prev[self.next[slot]] = slot;
                }

                if cur == self.tail {
                    self.tail = slot
                }
            }
        }
        self.len += 1;
    }

    pub fn front_time(&self) -> Option<Duration> {
        if
        /*self.is_empty()*/
        self.head == usize::MAX {
            None
        } else {
            Some(self.time[self.head])
        }
    }

    pub fn pop_min(&mut self) -> Option<(E, Duration, usize)> {
        if self.is_empty() {
            None
        } else {
            let ele = self.head;
            self.head = self.next[ele];
            self.len -= 1;

            if self.head == usize::MAX {
                self.tail = usize::MAX;
            } else {
                self.prev[self.head] = usize::MAX;
            }

            let res = (self.body[ele].take().unwrap(), self.time[ele], self.id[ele]);
            self.time[ele] = Duration::ZERO;
            self.id[ele] = 0;

            self.free_list.push(ele);
            Some(res)
        }
    }

    pub(super) fn info(&self) -> String {
        format!(
            "CLL {{ len: {}, head: {}, tail: {}, prev: {:?}, next: {:?}, free_list: {:?} }}",
            self.len, self.head, self.tail, self.prev, self.next, self.free_list
        )
    }

    pub fn grow(&mut self) {
        let old_cap = self.next.len();
        self.next.resize(2 * old_cap, usize::MAX);
        self.prev.resize(2 * old_cap, usize::MAX);
        self.time.resize(2 * old_cap, Duration::ZERO);
        self.id.resize(2 * old_cap, usize::MAX);
        self.body.resize_with(2 * old_cap, || None);
        self.free_list
            .append(&mut (old_cap..self.next.len()).collect())
    }

    pub(super) fn iter(&self) -> Iter<'_, E> {
        self.into_iter()
    }

    #[allow(unused)]
    pub(super) fn iter_mut(&mut self) -> IterMut<'_, E> {
        self.into_iter()
    }
}

impl<E: Debug> Debug for CacheOptimizedLinkedList<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkedList")
            .field("len", &self.len)
            .field("cap", &self.prev.len())
            .field("head", &self.head)
            .field("tail", &self.tail)
            .field(
                "entries",
                &(0..self.prev.len())
                    .map(|i| {
                        if self.free_list.contains(&i) {
                            "-".to_string()
                        } else {
                            format!(
                                "i = {:?} ({} .. {})",
                                self.body[i].as_ref().unwrap(),
                                self.prev[i],
                                self.next[i]
                            )
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .field("free_list", &self.free_list)
            .finish()
    }
}

impl<E: PartialEq> PartialEq for CacheOptimizedLinkedList<E> {
    fn eq(&self, other: &Self) -> bool {
        let mut lhs = self.iter();
        let mut rhs = other.iter();
        loop {
            match (lhs.next(), rhs.next()) {
                (Some(l), Some(r)) => {
                    if l != r {
                        return false;
                    }
                }
                (None, None) => break,
                _ => return false,
            }
        }
        true
    }
}

impl<E: Eq> Eq for CacheOptimizedLinkedList<E> {}

impl<E, const N: usize> From<[(E, Duration); N]> for CacheOptimizedLinkedList<E> {
    fn from(value: [(E, Duration); N]) -> Self {
        Self::from_iter(value)
    }
}

impl<E> FromIterator<(E, Duration)> for CacheOptimizedLinkedList<E> {
    fn from_iter<T: IntoIterator<Item = (E, Duration)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut r = Self::with_capacity(iter.size_hint().0);
        for (event, time) in iter {
            r.add(event, time, 0)
        }
        r
    }
}

pub struct Iter<'a, E> {
    dll: &'a CacheOptimizedLinkedList<E>,
    cur: usize,
}

impl<'a, E> Iterator for Iter<'a, E> {
    type Item = (&'a E, &'a Duration);

    fn next(&mut self) -> Option<Self::Item> {
        // SAFTEY:
        // Will point to a valid node since:
        // IA) head->next is a valid node
        // IS) each time the next node is check to be non-null (thus valid)
        let cur = self.cur;
        let result: Option<(*const E, *const Duration)> = {
            if cur == usize::MAX {
                // is tail
                None
            } else {
                // SAFTEY:
                // cur is allways valid + now non-tail
                self.cur = self.dll.next[cur];
                Some((
                    unsafe { self.dll.body[cur].as_ref().unwrap_unchecked() },
                    &self.dll.time[cur],
                ))
            }
        };
        result.map(|(v, t)| unsafe { (&*v, &*t) })
    }
}

impl<'a, E> IntoIterator for &'a CacheOptimizedLinkedList<E> {
    type Item = (&'a E, &'a Duration);
    type IntoIter = Iter<'a, E>;
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            dll: self,
            cur: self.head,
        }
    }
}

pub struct IterMut<'a, E> {
    dll: &'a mut CacheOptimizedLinkedList<E>,
    cur: usize,
}

impl<'a, E> Iterator for IterMut<'a, E> {
    type Item = (&'a mut E, &'a Duration);

    fn next(&mut self) -> Option<Self::Item> {
        // SAFTEY:
        // Will point to a valid node since:
        // IA) head->next is a valid node
        // IS) each time the next node is check to be non-null (thus valid)
        let cur = self.cur;
        let result: Option<(*mut E, *const Duration)> = {
            if cur == usize::MAX {
                // is tail
                None
            } else {
                self.cur = self.dll.next[cur];
                // SAFTEY:
                // cur is allways valid + now non-tail
                Some((
                    unsafe { self.dll.body[cur].as_mut().unwrap_unchecked() },
                    &self.dll.time[cur],
                ))
            }
        };
        result.map(|(v, t)| unsafe { (&mut *v, &*t) })
    }
}

impl<'a, E> IntoIterator for &'a mut CacheOptimizedLinkedList<E> {
    type Item = (&'a mut E, &'a Duration);
    type IntoIter = IterMut<'a, E>;
    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            cur: self.head,
            dll: self,
        }
    }
}

pub struct IntoIter<E> {
    dll: CacheOptimizedLinkedList<E>,
}

impl<E> Iterator for IntoIter<E> {
    type Item = (E, Duration);
    fn next(&mut self) -> Option<Self::Item> {
        let (e, t, _) = self.dll.pop_min()?;
        Some((e, t))
    }
}

impl<E> IntoIterator for CacheOptimizedLinkedList<E> {
    type Item = (E, Duration);
    type IntoIter = IntoIter<E>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { dll: self }
    }
}
