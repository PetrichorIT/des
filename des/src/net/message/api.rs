use crate::{
    net::{
        gate::IntoModuleGate,
        message::Message,
        module::with_mod_ctx,
        runtime::{buf_schedule_at, buf_send_at},
    },
    time::{Duration, SimTime},
};

///
/// Sends a message onto a given gate. This operation will be performed after
/// `handle_message` finished.
///
#[allow(clippy::needless_pass_by_value)]
pub fn send(msg: impl Into<Message>, gate: impl IntoModuleGate) {
    self::send_at(msg, gate, SimTime::now());
}

///
/// Sends a message onto a given gate with a delay. This operation will be performed after
/// `handle_message` finished.
///
#[allow(clippy::needless_pass_by_value)]
pub fn send_in(msg: impl Into<Message>, gate: impl IntoModuleGate, dur: Duration) {
    let deadline = SimTime::now() + dur;
    self::send_at(msg, gate, deadline);
}
///
/// Sends a message onto a given gate at the sepcified time. This operation will be performed after
/// `handle_message` finished.
///
/// # Panics
///
/// Panics if the send time is in the past.
///
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

///
/// Enqueues a event that will trigger the
/// [`Module::handle_message`](crate::net::module::Module::handle_message)
/// function in duration seconds, shifted by the processing time delay.
///
pub fn schedule_in(msg: impl Into<Message>, dur: Duration) {
    self::schedule_at(msg, SimTime::now() + dur);
}

///
/// Enqueues a event that will trigger the
/// [`Module::handle_message`](crate::net::module::Module::handle_message)
/// function at the given `SimTime`
///
/// # Panics
///
/// Panics if the specified time is in the past.
///
pub fn schedule_at(msg: impl Into<Message>, arrival_time: SimTime) {
    assert!(arrival_time >= SimTime::now());
    let msg: Message = msg.into();

    // plugin capture
    // let Some(msg) = plugin::plugin_output_stream(msg) else {
    //     return
    // };

    buf_schedule_at(msg, arrival_time);
}
