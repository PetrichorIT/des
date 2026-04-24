- add abstract gates
  - e.g. abstract gate ports[?], implies that any connection attempt to this gate id will create a new instance port[i]
  - OR allow for multiplexing gates (bad idea since that destroys graph properties)
- rework spawner / stereotype
- PropError(<io>) could use better variants if Error was defined upstream ... is the des_utils construction even resonable?
- add property to event: relevant (is_empty ignores non-relevant events)
  - can be used to ignore infinite timer events
- maybe depc send_in(<delay>) since it does not play nice with send error


### Ideas

Fundamental reconstruct

struct Host {
  handlers: Set<fn(MessageType) -> Response>
}

Host::new().with(ipv4_handler).with(ipv6_handler).with(tcp_handler);

// Requires an incoming Ipv4Packet at a given gate.
fn ipv4_handler(incoming: Ipv4Packet, state: State<Ipv4Fwd>, gate: GateRef, dispatch: &Dispatcher) {
  ...
  dispatch.send(Ipv4Packet); // < External (buffers)
  ...
  dispatch.delegate_to(TcpPacket); // < Internal send to other handler (fails if not handler matches the type)
}

trait Handler {
  fn reset(&mut self);
  fn call(&mut self, message: &Message, state: &InternalState);
}

> Remove the requirement for static-bound things like current() or BUF_CTX

Equivalence to current features:
- Signals (path-less delay-less packets -> maybe model as normal messages?)
- shutdown / startup (can be represented)
- spawner -> no reason not to (model as part of dispatcher?)
- props -> Part of state (conditional) or Props (custom extractor)


# v6.4

### Issues

1) Current unwind behaviour is inconsistent between sync and async 
   -> will never be simple as long as this distinction exist

2) Runtime/Builder is never used for other cases than Sim -> remove complexity

3) Tokio Time integration

### Solutions

1)
Remove the concept of sync-modules: async task-groups by default
-> perhaps provide sync-module for backward compatibility
-> main receive loop either via mspc::channel or seperate API (async fn current().recv(); maybe filtered by gate)

trait Module {
  start()
  end()
}

trait SyncModule {}
impl Module for SyncModule {}

-> just async error model (catch, crash, restart, report_as)

> Dropped

2)
See branch "integrated-runtime"

> Done

3)
needs tokio PR

> Later

4)
Decide how to access abstract gates (maybe using gate(...) should already clone?)

gate(<name>) will resolve to either a single-gate-cluster or an abstract gate (if present)
  -> abstract gate will be prioritized
  gate(<name>) if <name> abstract = <abstrac gate>
  gate(<name>) if <name> has only one gate = <that gate>
  game(<name>) if <name> namespace = error
  game(<name>) else = error
  
gate(<name>, <pos>) will always resolve to a c agte

Sim::gate maintains that behaviour

trait IntoModuleTree for most gate APIs ?
-> or just to resolve send(...) APIs
-> into gate?

> Done
