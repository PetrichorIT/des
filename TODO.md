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

# Change parent / child to Mrc dyn traits instead of unsafe calls

only when the performace hit is neglecable

# NET IPV6

With features 'net' 'cqueue' and 'internal-metrics' adding 'ipv6' to t-ndl
changes the overflow heap usage from avg 0 to avg 180

investigate
