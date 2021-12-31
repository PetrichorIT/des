# Core rework

Invariants:
User API relmains mostly stable

Target:
des_core::core
des_core::net (partially)

# Concepts

use non dyn types for event simulation.

Runtime<A, E>
where

-   A: Application
-   E: EventEnum

By E: Sized non event boxing is nessecary since EventNode<E> is inherintly Sized

```rust
#[derive(SimEvents)]
enum SimEvents {
    A(EventAHandler),
    B(EventBHandler),
}

// macro def
impl SimEvents {
    fn handle(self) { /*...*/ }
}

trait EventHandler {
    fn handle(self)
}
```

By proviing traits indirectly Boxing can be prevented since SimEvents guarantees that all used type
are Sized , which they are inherntinly.
Macros can do the heavy lifting.

# TODOS

-   new Event core primitives
-   util_macros
-   rt rework
-   net update to enure compatibility

# Additional ideas

-   SimTime rework?
-   bettern interning integration
