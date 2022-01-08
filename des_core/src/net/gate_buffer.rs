#[cfg(not(feature = "static_gates"))]
pub type GateBuffer = dynamic_gate_buffer::GateBuffer;
#[cfg(not(feature = "static_gates"))]
pub type GateRef = dynamic_gate_buffer::GateRef;

#[cfg(not(feature = "static_gates"))]
mod dynamic_gate_buffer {
    use crate::net::*;
    use crate::util::*;

    ///
    /// A global buffer for all gates to speed up search times
    ///
    pub struct GateBuffer {
        inner: Vec<Gate>,
        gen: usize,
    }

    impl GateBuffer {
        ///
        /// Creates a new empty buffer.
        ///
        pub fn new() -> Self {
            Self {
                inner: Vec::new(),
                gen: 0,
            }
        }

        ///
        /// Creates a buffer with the given capacity
        ///
        pub fn with_capacity(cap: usize) -> Self {
            Self {
                inner: Vec::with_capacity(cap),
                gen: 0,
            }
        }

        ///
        /// Inserts a element into the bufferö
        ///
        pub fn insert(&mut self, gate: Gate) -> GateRef {
            let id = gate.id();
            let insert_at = match self.inner.binary_search_by_key(&gate.id(), |c| c.id()) {
                Ok(insert_at) | Err(insert_at) => insert_at,
            };

            self.inner.insert(insert_at, gate);
            self.gen += 1;

            GateRef::new(id, self)
        }

        ///
        /// Extracts a element identified by id, using binary search.
        ///
        pub fn gate(&self, id: GateId) -> Option<&Gate> {
            let pos = match self.inner.binary_search_by_key(&id, |c| c.id()) {
                Ok(pos) => pos,
                Err(_) => return None,
            };

            Some(&self.inner[pos])
        }

        ///
        /// Extracts a element mutably identified by id, using binary search.
        ///
        pub fn gate_mut(&mut self, id: GateId) -> Option<&mut Gate> {
            let pos = match self.inner.binary_search_by_key(&id, |c| c.id()) {
                Ok(pos) => pos,
                Err(_) => return None,
            };

            Some(&mut self.inner[pos])
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

    impl Default for GateBuffer {
        fn default() -> Self {
            Self::new()
        }
    }

    ///
    /// A reference to a [Gate] stored in a [GateBuffer].
    ///
    /// # Note
    ///
    /// This object will fetch the Gate by performing a binary search
    /// (via the GateBuffer::gate method) an then returning the found result.
    /// Note that this direct reference will be stored together with a gen
    /// to make furture binary search unnessecary as long as the buffer doe
    /// not change (gen does not change).
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

        // Internal fn to hide caching calls.
        #[allow(clippy::mut_from_ref)]
        fn direct(&self) -> &mut Option<(usize, *mut Gate)> {
            //
            // # Safty
            //
            // This is just a call to hide interiour mut of cached values.
            //
            unsafe { &mut *self.direct.get() }
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
            let buffer_gen = buffer.gen;

            //
            // # Safty
            //
            // This is safe since the ptr was created from a valid instance
            // in a previous call of this fn, and the refernced buffer has not changed
            // as indicated by gen.
            //
            if let Some((gen, ptr)) = self.direct() {
                if *gen == buffer_gen {
                    return unsafe { &**ptr };
                }
            }

            let gate = buffer.gate(self.id).unwrap();

            //
            // # Safty
            //
            // This is save since only self may mutate gen and this simulation is single-threaded.
            //
            let r = unsafe { &mut *self.direct.get() };
            *r = Some((buffer_gen, (gate as *const Gate) as *mut Gate));

            gate
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
            let buffer_gen = buffer.gen;

            //
            // # Safty
            //
            // This is safe since the ptr was created from a valid instance
            // in a previous call of this fn, and the refernced buffer has not changed
            // as indicated by gen.
            //
            if let Some((gen, ptr)) = self.direct() {
                if *gen == buffer_gen {
                    return unsafe { &mut **ptr };
                }
            }

            let gate = buffer.gate_mut(self.id).unwrap();

            //
            // # Safty
            //
            // This is save since only self may mutate gen and this simulation is single-threaded.
            //
            let r = unsafe { &mut *self.direct.get() };
            *r = Some((buffer_gen, gate));

            gate
        }
    }
}

#[cfg(feature = "static_gates")]
pub type GateBuffer = static_gate_buffer::GateBuffer;
#[cfg(feature = "static_gates")]
pub type GateRef = static_gate_buffer::GateRef;

#[cfg(feature = "static_gates")]
mod static_gate_buffer {
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
}
