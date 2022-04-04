# Reevaluate interning system

Try using std-close implementation of refenenc counting in
combination with delayed-drop type-specific tables.

GlobalInerner {
Interner(Packet)
Interner(String)
}

Also consider Cow prt vs Rc ptr

# Rework pkt-headers msg size calculations

# Check ParHandle costs of std::cell::RefCell

# Channel optional cost field.
