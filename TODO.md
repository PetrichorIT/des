# Typed interning using Rc or Arc

for interning use a two layered structure
GlobalInerner {
Interner(Packet)
Interner(String)
}

therby index pairs are used (t, i)
and t implicitly defines the type
leading to correct drops

using raw ptrs the global interner can ignore
substructure generics:

# General interning overhaul

try to redesign interning,
also use Clone-on-write for typed instances

# Deprecated Channel ID

since channels are managed using Mrc not central buffers, ids became obsolete
