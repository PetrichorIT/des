# Dropping of net structures if needed.

### Channels

Channels are controled using reference counting from two entities: the

-   Gates
-   Subsystems

If all gates and all subystems are dropped gates are trivialy dropped as well.

### Gates

Gates are structured in a one-directional chain with Weak backwards pointers.
Thus a gate is dropped if:

-   The gate before its is dropped, means the first gate in the chain is dropped.
-   AND the owners of the gates are dropped

Thus it must only be guranteed that all modules are dropped.
AND all subsystems.

# Module drop order

Hierichally from top to bottom. However ModuleRef hold Arcs so there is a cylic dependency.
Thus use Weak Arcs on the following edges:

-   Child to parent ptr
-   Gate to owner ptr
