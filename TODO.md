# Remove GateType

Remove gate type because its an unused attribute.
Add runtime check for exclusive gate usage.

# Remmove channel nessacity.

Remove channel metioning from relevant gates and add channeling at end of gate, not end of gate chain.

This means in an A -> SA -> SB -> B
config only SA has a channel ref so GateArrivalEvent(A) is deferred while
HandleMessageAtModuleEvent is not

# Add attribute macro for module creation

This should "remove" the need for module_core() and module_core_mut() impls

# Refactor internal memory and index management

Use Interning similar to SourceMap and central registrys
