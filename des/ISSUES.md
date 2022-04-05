# Reevaluate interning system

Try using std-close implementation of refenenc counting in
combination with delayed-drop type-specific tables.

GlobalInerner {
Interner(Packet)
Interner(String)
}

Also consider Cow prt vs Rc ptr
