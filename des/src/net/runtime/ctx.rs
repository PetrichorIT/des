#![allow(missing_docs)]

use log::info;

use super::{HandleMessageEvent, MessageAtGateEvent, NetworkRuntime, NetworkRuntimeGlobals};
use crate::net::module::{MOD_CTX, SETUP_FN};
use crate::net::{gate::GateRef, message::Message, NetEvents};
use crate::prelude::{module_id, EventLifecycle, GateServiceType, ModuleRef};
use crate::runtime::Runtime;
use crate::sync::Mutex;
use crate::time::SimTime;
use std::sync::{Arc, Weak};

static BUF_CTX: Mutex<BufferContext> = Mutex::new(BufferContext {
    events: Vec::new(),
    loopback: Vec::new(),
    shutdown: None,
    globals: None,
});

type LoopbackBuffer = Vec<(Message, SimTime)>;

struct BufferContext {
    // All new events that will be scheduled
    events: Vec<(NetEvents, SimTime)>,

    // (Message, SendTime)
    loopback: LoopbackBuffer,
    // shudown,
    #[allow(clippy::option_option)]
    shutdown: Option<Option<SimTime>>,
    // globals
    globals: Option<Weak<NetworkRuntimeGlobals>>,
}

unsafe impl Send for BufferContext {}
unsafe impl Sync for BufferContext {}

///
/// Returns the globals of the runtime.
///
/// # Panics
///
/// This function panics if the no runtime is currently active.
/// Note that a runtime is active if a instance of [`NetworkRuntime`] exists.
///
#[must_use]
pub fn globals() -> Arc<NetworkRuntimeGlobals> {
    let ctx = BUF_CTX.lock();
    ctx.globals
        .as_ref()
        .unwrap()
        .upgrade()
        .expect("No runtime globals attached")
}

pub(crate) fn buf_send_at(mut msg: Message, gate: GateRef, send_time: SimTime) {
    let mut ctx = BUF_CTX.lock();
    msg.header.sender_module_id = module_id();

    let inital_token = gate.owner().logger_token;

    // (0) If delayed send is active, dont skip gate_refs
    if send_time > SimTime::now() {
        ctx.events.push((
            NetEvents::MessageAtGateEvent(MessageAtGateEvent { gate, message: msg }),
            send_time,
        ));
        return;
    }

    // (1) Follow the gate chain until either the end or a channel is reached.
    let mut current_gate = gate;
    while let Some(next_gate) = current_gate.next_gate() {
        log_scope!(current_gate.owner().ctx.logger_token);

        // a next gate exists, so forward to the next gate allready
        msg.header.last_gate = Some(GateRef::clone(&next_gate));

        info!(
            "Gate '{}' forwarding message [{}] to next gate delayed: {}",
            current_gate.name(),
            msg.str(),
            current_gate.channel().is_some()
        );

        if let Some(ch) = current_gate.channel_mut() {
            // Channel delayed connection
            assert!(
                current_gate.service_type() != GateServiceType::Input,
                "Channels cannot start at a input node"
            );

            ch.send_message(msg, &next_gate, &mut ctx.events);
            log_scope!(inital_token);

            return;
        } else {
            // We can skip this bridge since it is only a symbolic link
            current_gate = next_gate;
        }
    }

    debug_assert!(current_gate.next_gate().is_none());
    log_scope!(current_gate.owner().ctx.logger_token);

    assert!(
        current_gate.service_type() != GateServiceType::Output,
        "Messages cannot be forwarded to modules on Output gates. (Gate '{}' owned by Module '{}')",
        current_gate.str(),
        current_gate.owner().as_str()
    );

    info!(
        "Gate '{}' forwarding message [{}] to module #{}",
        current_gate.name(),
        msg.str(),
        current_gate.owner().ctx.id
    );

    let module = current_gate.owner();
    ctx.events.push((
        NetEvents::HandleMessageEvent(HandleMessageEvent {
            module,
            message: msg,
        }),
        SimTime::now(),
    ));

    log_scope!(inital_token);
}

pub(crate) fn buf_schedule_at(msg: Message, arrival_time: SimTime) {
    // continue to delay the delivery of event, since non other components are
    // used, and we dont block any channels. additionally this ensures that
    // timeouts are allways ordered later than packets, which is good
    let mut ctx = BUF_CTX.lock();
    ctx.loopback.push((msg, arrival_time));
}

pub(crate) fn buf_schedule_shutdown(restart: Option<SimTime>) {
    let mut ctx = BUF_CTX.lock();
    ctx.shutdown = Some(restart);
}

pub(crate) fn buf_set_globals(globals: Weak<NetworkRuntimeGlobals>) {
    let mut ctx = BUF_CTX.lock();
    ctx.globals = Some(globals);

    // SAFTEY:
    // reseting the MOD_CTX is safe, since simulation lock is aquired.
    unsafe {
        MOD_CTX.reset(None);
    }
}

pub(crate) fn buf_process<A>(module: &ModuleRef, rt: &mut Runtime<NetworkRuntime<A>>)
where
    A: EventLifecycle<NetworkRuntime<A>>,
{
    let mut ctx = BUF_CTX.lock();

    // # NEW
    for (event, time) in ctx.events.drain(..) {
        rt.add_event(event, time)
    }

    // Send loopback events from 'scheduleAt'
    for (message, time) in ctx.loopback.drain(..) {
        rt.add_event(
            NetEvents::HandleMessageEvent(HandleMessageEvent {
                module: module.clone(),
                message: message,
            }),
            time,
        );
    }

    // MARKER: shutdown
    if let Some(rest) = ctx.shutdown.take() {
        use crate::net::message::TYP_RESTART;

        log::debug!("Shuttind down module and restaring at {:?}", rest);
        module
            .ctx
            .active
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // drop the rt, to prevent all async activity from happening.
        #[cfg(feature = "async")]
        drop(module.ctx.async_ext.write().rt.take());

        // drop all hooks to ensure all messages reach the async impl
        // module.ctx.hooks.borrow_mut().clear(); TODO: Plugin clean
        module.ctx.plugins.write().clear();
        SETUP_FN.read()(&module.ctx);

        // Reschedule wakeup
        if let Some(rest) = rest {
            rt.add_event(
                NetEvents::HandleMessageEvent(HandleMessageEvent {
                    module: module.clone(),
                    message: Message::new().typ(TYP_RESTART).build(),
                }),
                rest,
            );
        }
    }
}
