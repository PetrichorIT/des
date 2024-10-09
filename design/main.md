# v0.6 proposal

- Seperate Module definitions from simulation building
  - `trait Module` should only be concerened with the event handeling of modules
  - `NetworkApplication` should not create owned instances of an item
  - turmoil inspiered API with `node(path, something dyn Module)`
- NDL derivation acts as instructiosn for a builder object
  - non hierachical builders, all from top level
  - automatic parent detection (required)
  - automatic dup detection
  
### Questions

- `reset` when `dyn Module` does not allow for the usage of `new` (maybe just `reset` is good)
- can `Fn(Message) -> ()` or `Fn() -> Fn(Message) -> ()` be used as restartable modules (similar to `client` / `host`)
- async by default
  - async API only on feature `async`
  - without feataure `async` but no wakeup RT to allow internal async to be used
  - could remove the need for indicate_async
  - requires `handler.poll(&NEVER_WAKEUP_CTX)` without feature `async`

- How to implement plugins
  - same same ?

### Ideas

- `sim.node(...)` could not only accept `T: Module` but `T: Modules` -> creates multiple submodules based on top-level module
```rust
struct Node;
impl Modules for Node {
    fn build(&self, sim: ScopedSim<'a>) {
        sim.node("dns-server", ...);
        sim.node("watchdog", ...);
        sim.node("router", ...);

        sim.gate("dns-server", "port");
        // ...
    }
}

fn main() {
    let sim = NetworkApplication::new();
    sim.node("a", Node);

    // Will create
    // `a`
    // `a.dns-server`
    // `a.watchdog` ...
    // including internal gate connections
}


```

### Code example

```rust
fn main() {
    let sim = NetworkApplication::new(());
    sim.node("alice", || {
        let mut state = 123;
        |msg| {
            println!("{msg:?}");
            state += 1;
        }}
    );
    sim.gate("alice", "port");


    struct Bob;
    impl Module for Bob {
        fn handle(&mut self, msg: Message) {
            println!("{msg:?}");
        }
    }
    sim.node("bob", Bob);
    sim.gate("bob", "port");

    sim.connect("alice.port", "bob.port");
    // sim.connect_with(..., channel)s

    let rt = Builder::seeded(123).build(sim);

}
```

> NDL

```rust
fn main() {
    let mut sim = NdlApplication::new("path");
    sim.registry(registry![]);

    // or
    sim.register("NDLName", |path| { ... dyn Module });


    sim.build() // only start calls here, since gates require allready existing handlers
}

```


## Advantages

- No `Module::new` so automatic trait object safety
- Since instances are created in the builder, custom constructors
- No typing bullshit, `parent / child` with castings should be removed

## Disadvantages

- `NDL` support requires some generic constructor
- Custom builder support, so `trait Module` and `trait HowToBuildeAModule` required for more complex components