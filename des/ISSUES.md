# Reevaluate interning system

Try using std-close implementation of refenenc counting in
combination with delayed-drop type-specific tables.

GlobalInerner {
Interner(Packet)
Interner(String)
}

Also consider Cow prt vs Rc ptr

# Change module-level parent-child system to use Mrc

This change must allow checked casts, be negleable on performace
and guranatee refence intergrity
