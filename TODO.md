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


# Add freeze to EXEC_CTX / BUF_CTX to prevent accidental uses

# Make separation between SystemEvent - EventExecContext - NodeContext

- Idea: conisder these seperate components  
  - NodeContext -> current(), resolve of calls regarding location within the topo
  - EventExecContext -> collect created events, basic unit of work
  - SystemEvent -> could encompass more that one event to coordinate events (SimStart)

- Relations (Exec:Node)
  - 1:1 most common
  - 0:1 new / reset, no generation of any sim-events allowed
  - 1:0 impossible i think, thats just external view
  - 1:n calling APIs
  - n:1 useless, thats just multiple events for the same node
- ExecutionContext as part of current()
