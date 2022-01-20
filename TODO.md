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
substructure generics°:
