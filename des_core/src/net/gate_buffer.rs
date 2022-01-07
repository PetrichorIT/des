use crate::net::*;
use crate::util::*;

///
/// A global buffer for all gates to speed up search times
///
pub struct GateBuffer {
    inner: SyncCell<Vec<Gate>>,
    gen: usize,
}

impl GateBuffer {
    fn inner(&self) -> &Vec<Gate> {
        unsafe { &*self.inner.get() }
    }

    fn inner_mut(&mut self) -> &mut Vec<Gate> {
        unsafe { &mut *self.inner.get() }
    }

    ///
    /// Creates a new empty buffer.
    ///
    pub fn new() -> Self {
        Self {
            inner: SyncCell::new(Vec::new()),
            gen: 0,
        }
    }

    ///
    /// Creates a buffer with the given capacity
    ///
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: SyncCell::new(Vec::with_capacity(cap)),
            gen: 0,
        }
    }

    ///
    /// Inserts a element into the bufferö
    ///
    pub fn insert(&mut self, gate: Gate) -> GateRef {
        let id = gate.id();
        let insert_at = match self.inner().binary_search_by_key(&gate.id(), |c| c.id()) {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        self.inner_mut().insert(insert_at, gate);
        self.gen += 1;

        GateRef::new(id, self)
    }

    ///
    /// Extracts a element identified by id, using binary search.
    ///
    pub fn gate(&self, id: GateId) -> Option<&Gate> {
        let pos = match self.inner().binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&self.inner()[pos])
    }

    ///
    /// Extracts a element mutably identified by id, using binary search.
    ///
    pub fn gate_mut(&mut self, id: GateId) -> Option<&mut Gate> {
        let pos = match self.inner().binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&mut self.inner_mut()[pos])
    }

    ///
    /// Retrieves a target gate of a gate chain.
    ///
    pub fn gate_dest(&self, source_id: GateId) -> Option<&Gate> {
        let mut gate = self.gate(source_id)?;
        while gate.id() != GATE_SELF {
            gate = self.gate(gate.next_gate())?
        }
        Some(gate)
    }
}

///
/// A refernece to a [Gate] stored in a [GateBuffer].
///
/// # Note
///
/// This is modelled a a id and ptr to the buffer not the element
/// itself since the elemnt may move at other inserts or realloc.
///
#[derive(Debug, Clone)]
pub struct GateRef {
    id: GateId,
    buffer: *mut GateBuffer,

    direct: SyncCell<Option<(usize, *mut Gate)>>,
}

impl GateRef {
    ///
    /// Creates from raw instance
    ///
    pub fn new(id: GateId, buffer: &mut GateBuffer) -> Self {
        Self {
            id,
            buffer,
            direct: SyncCell::new(None),
        }
    }

    fn direct(&self) -> &mut Option<(usize, *mut Gate)> {
        unsafe { &mut *self.direct.get() }
    }

    pub fn get(&self) -> &Gate {
        let buffer = unsafe { &*self.buffer };
        let buffer_gen = buffer.gen;

        if let Some((gen, ptr)) = self.direct() {
            if *gen == buffer_gen {
                return unsafe { &**ptr };
            }
        }

        let gate = buffer.gate(self.id).unwrap();

        let r = unsafe { &mut *self.direct.get() };
        *r = Some((buffer_gen, (gate as *const Gate) as *mut Gate));

        gate
    }

    pub fn get_mut(&mut self) -> &mut Gate {
        let buffer = unsafe { &mut *self.buffer };
        let buffer_gen = buffer.gen;

        if let Some((gen, ptr)) = self.direct() {
            if *gen == buffer_gen {
                return unsafe { &mut **ptr };
            }
        }

        let gate = buffer.gate_mut(self.id).unwrap();

        let r = unsafe { &mut *self.direct.get() };
        *r = Some((buffer_gen, gate));

        gate
    }
}
