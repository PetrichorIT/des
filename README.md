# DES - Discrete event simulator

DES is a discrete event simulator inspired by OMNet++, written in Rust.
Accordingly it can (in the future) be used to simulate networks using convient tools
like custom description languages, automatic memory controll and a strongly abstracted
programming interface.

## IDEAS

- configurable message T for custom base messages
- static <T> with type erased holder for agnostic api
- baseloader contents as sim builder parameter

## V6.2 Redesign

- two main APIs: (sync) handle_message(msg) / (async) launch(rx)
  - base (sync) for event processing
  - no async at_sim_start
  - join at shutdown mechanic (for main task? or for abitrary tasks?)

- Plugin szenario
  - "i depend of functionality of ctx plugin A"
  - a) plugins do not change base API of modules, so only stream processing
  - b) plugins do change the base API, -> wrapper around `Module`?

  trait InetModule {
    async fn launch(&mut self)
  }

  // Not possible with orphan rules
  impl<T: InetModule> Module for InetModule { ... }
  impl<T: InetModule> ModuleBlock for InetModule { ... }

  // possible?
  struct Wrapper(I: InetModule)
  impl Module for Wrapper<I> {...}
