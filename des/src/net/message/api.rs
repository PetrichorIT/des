use crate::{
    net::{
        gate::IntoModuleGate,
        message::Message,
        module::with_mod_ctx,
        runtime::{buf_schedule_at, buf_send_at},
    },
    time::{Duration, SimTime},
};

/// Sends a message onto a given gate. The effects of this sending operation
/// will be observable directly, so an attached channel will be busy right after
/// the the call to `send`.
///
/// > *This function requires a node-context within the simulation*
///
/// # Examples
///
/// ```
/// # use des::prelude::*;
/// struct MyModule;
///
/// impl Module for MyModule {
///     fn handle_message(&mut self, _msg: Message) {
///         send(
///             Message::new().id(123).content("Hello world").build(),
///             "out"
///         );
///     }
/// }
///
/// let mut sim = Sim::new(());
/// sim.node("alice", MyModule);
/// sim.gate("alice", "out");
/// /* ... */
///
/// let _ = Builder::new().build(sim).run();
/// ```
#[allow(clippy::needless_pass_by_value)]
pub fn send(msg: impl Into<Message>, gate: impl IntoModuleGate) {
    self::send_at(msg, gate, SimTime::now());
}

/// Sends a message onto a given gate with a delay. If the delay is nonzero
/// the effects will only be observable later on.
///
/// > *This function requires a node-context within the simulation*
///
/// # Examples
///
/// ```
/// # use des::prelude::*;
/// # struct SomeOtherModule;
/// # impl Module for SomeOtherModule {}
/// struct MyModule;
///
/// impl Module for MyModule {
///     fn at_sim_start(&mut self, _: usize) {
///         send_in(Message::new().kind(42).build(), "out", Duration::from_secs(2));
///         assert!(
///             !current()
///                 .gate("out", 0).unwrap()
///                 .path_iter().next().unwrap()
///                 .channel().unwrap()
///                 .is_busy()     
///        );
///     }
/// }
///
/// let mut sim = Sim::new(());
/// sim.node("alice", MyModule);
/// let out_gate = sim.gate("alice", "out");
///
/// sim.node("bob", SomeOtherModule);
/// let in_gate = sim.gate("bob", "in");
///
/// out_gate.connect(in_gate, Some(
///     /* Channel definition */
///     # Channel::new(ChannelMetrics { bitrate: 10000, jitter: Duration::ZERO, latency: Duration::from_millis(10), drop_behaviour: ChannelDropBehaviour::Drop })
/// ));
///
/// let _ = Builder::new().build(sim).run();
///
/// ```
#[allow(clippy::needless_pass_by_value)]
pub fn send_in(msg: impl Into<Message>, gate: impl IntoModuleGate, dur: Duration) {
    let deadline = SimTime::now() + dur;
    self::send_at(msg, gate, deadline);
}
/// Sends a message onto a given gate at the specific time. This operation is
/// equivalent to [`send_in`].
///
/// > *This function requires a node-context within the simulation*
///
/// # Panics
///
/// Panics if the send time is in the past.
#[allow(clippy::needless_pass_by_value)]
pub fn send_at(msg: impl Into<Message>, gate: impl IntoModuleGate, send_time: SimTime) {
    assert!(send_time >= SimTime::now());
    // (0) Cast the message.
    let msg: Message = msg.into();

    let gate = with_mod_ctx(|ctx| {
        // (1) Cast the gate
        #[allow(clippy::explicit_auto_deref)] // IS RIGHT ?
        gate.as_gate(ctx)
    });

    if let Some(gate) = gate {
        // plugin capture
        // let Some(msg) = plugin::plugin_output_stream(msg) else {
        //     return
        // };

        buf_send_at(msg, gate, send_time);
    } else {
        #[cfg(feature = "tracing")]
        tracing::error!("Error: Could not find gate in current module");
    }
}

/// Enqueues a event that will trigger the
/// [`Module::handle_message`](crate::net::module::Module::handle_message)
/// function in duration seconds, shifted by the processing time delay.
///
/// > *This function requires a node-context within the simulation*
///
/// # Examples
///
/// ```
/// # use des::prelude::*;
/// struct Timer { period: Duration }
/// impl Module for Timer {
///     fn at_sim_start(&mut self, _: usize) {
///         schedule_in(Message::new().content("wakeup").build(), self.period);
///     }
///
///     fn handle_message(&mut self, msg: Message) {
///         assert_eq!(msg.try_content::<&str>(), Some(&"wakeup"));
///         /* Do something periodicly */
///         schedule_in(msg, self.period);
///     }
/// }
///
/// let mut sim = Sim::new(());
/// sim.node("timer", Timer { period: Duration::from_secs(5) });
/// /* ... */
///
/// let _ = Builder::new().max_time(100.0.into()).build(sim).run();
/// ```
pub fn schedule_in(msg: impl Into<Message>, dur: Duration) {
    self::schedule_at(msg, SimTime::now() + dur);
}

/// Enqueues a event that will trigger the
/// [`Module::handle_message`](crate::net::module::Module::handle_message)
/// function at the given `SimTime`. This operation is equivalent to [`schedule_in`].
///
/// > *This function requires a node-context within the simulation*
///
/// # Panics
///
/// Panics if the specified time is in the past.
pub fn schedule_at(msg: impl Into<Message>, arrival_time: SimTime) {
    assert!(arrival_time >= SimTime::now());
    let msg: Message = msg.into();

    // plugin capture
    // let Some(msg) = plugin::plugin_output_stream(msg) else {
    //     return
    // };

    buf_schedule_at(msg, arrival_time);
}
