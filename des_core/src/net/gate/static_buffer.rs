use crate::net::*;

///
/// A global buffer for all gates to speed up search times.
/// Uses static ids. After the buffer is locked no more elements may be inserted.
///
pub struct GateBuffer {
    inner: Vec<Option<Gate>>,
    locked: bool,
}

impl GateBuffer {
    ///
    /// Creates a new empty buffer.
    ///
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            locked: false,
        }
    }

    ///
    /// Creates a buffer with the given capacity
    ///
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),
            locked: false,
        }
    }

    ///
    /// Inserts a element into the bufferö
    ///
    pub fn insert(&mut self, gate: Gate) -> GateRef {
        assert!(!self.locked, "Cannot insert elements to locked buffer");
        let id = gate.id();
        let idx = gate.id().raw() - GateId::MIN.raw();
        let idx = idx as usize;

        if idx >= self.inner.len() {
            self.inner.resize(idx + 1, None);
        }

        self.inner[idx] = Some(gate);

        GateRef::new(GateId::from(id), self)
    }

    pub fn lock(&mut self) {
        println!("Locked buffer with {} gates", self.inner.len());
        self.locked = true;
    }

    ///
    /// Extracts a element identified by id, using binary search.
    ///
    pub fn gate(&self, id: GateId) -> Option<&Gate> {
        let value = &self.inner[(id.raw() - GateId::MIN.raw()) as usize];
        value.as_ref().map(|v| v)
    }

    ///
    /// Extracts a element mutably identified by id, using binary search.
    ///
    pub fn gate_mut(&mut self, id: GateId) -> Option<&mut Gate> {
        let value = &mut self.inner[(id.raw() - GateId::MIN.raw()) as usize];
        value.as_mut().map(|v| v)
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
}

impl GateRef {
    ///
    /// Creates from raw instance
    ///
    pub fn new(id: GateId, buffer: &mut GateBuffer) -> Self {
        Self { id, buffer }
    }

    pub fn get(&self) -> &Gate {
        //
        // # Safty
        //
        // This is safe since those functions will only be called as long
        // as the simulation is running, which implies that the NetworkRuntime
        // is still alive thereby its member 'gate_buffer' as well. Since GateBuffer
        // and NetworkRuntime are Sized there will be no reallocs.
        //
        let buffer = unsafe { &*self.buffer };
        buffer.gate(self.id).unwrap()
    }

    pub fn get_mut(&mut self) -> &mut Gate {
        //
        // # Safty
        //
        // This is safe since those functions will only be called as long
        // as the simulation is running, which implies that the NetworkRuntime
        // is still alive thereby its member 'gate_buffer' as well. Since GateBuffer
        // and NetworkRuntime are Sized there will be no reallocs.
        //
        let buffer = unsafe { &mut *self.buffer };
        buffer.gate_mut(self.id).unwrap()
    }
}
