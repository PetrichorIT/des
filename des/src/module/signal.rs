use crate::{
    message::Body,
    module::{ModuleRefWeak, with_mod_ctx},
    prelude::ModuleRef,
    runtime::{NetEvents, SignalEvent},
    schedule_event,
    time::SimTime,
};

/// A globally unique identifier for a set of signals.
pub type SignalCode = usize;

/// An automatic signal that is being send once a module panics.
pub const SIGNAL_MODULE_PANICED: SignalCode = 0x1;
/// An automatic signal that sim start has been executed, received in the `Running` state.
pub const SIGNAL_SIM_START_DONE: SignalCode = 0x2;

/// A signal that is being propagated in a publish-subscribe pattern.
#[derive(Debug, Clone)]
pub struct Signal {
    /// The source of the signal.
    pub source: ModuleRef,
    /// A unqiue identifier for the signal
    pub code: SignalCode,
    /// Data carried along with the signal.
    pub body: Body,
}

/// Emits a signal to all subscribers
pub fn emit(signal: SignalCode, body: Body) {
    emit_at(signal, body, SimTime::now());
}

/// Emits a signal to all subscribers at a given time.
///
/// # Panics
///
/// This function panics if `at` is in the past.
pub fn emit_at(signal: SignalCode, body: Body, at: SimTime) {
    assert!(at >= SimTime::now(), "time travel is forbidden");

    let (me, subscribers) = with_mod_ctx(|ctx| {
        (
            ctx.me(),
            ctx.signal_subscribers
                .read()
                .get(&signal)
                .cloned()
                .unwrap_or_default(),
        )
    });

    let subscribers = subscribers
        .iter()
        .map(ModuleRefWeak::upgrade)
        .collect::<Option<Vec<_>>>()
        .expect("some referenced modules do no longer exist"); // < should that be a panic?

    if subscribers.is_empty() {
        return;
    }

    schedule_event(
        NetEvents::SignalEvent(SignalEvent {
            signal: Signal {
                source: me,
                code: signal,
                body,
            },
            subscribers,
        }),
        at,
    );
}
