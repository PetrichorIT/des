#![allow(missing_docs)]

use super::{
    HandleMessageEvent, MessageAtGateEvent, NetworkApplication, NetworkApplicationGlobals,
};
use crate::net::module::{MOD_CTX, SETUP_FN};
use crate::net::ModuleRestartEvent;
use crate::net::{gate::GateRef, message::Message, NetEvents};
use crate::prelude::{module_id, EventLifecycle, ModuleRef};
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
    globals: Option<Weak<NetworkApplicationGlobals>>,
}

unsafe impl Send for BufferContext {}
unsafe impl Sync for BufferContext {}

///
/// Returns the globals of the runtime.
///
/// # Panics
///
/// This function panics if the no runtime is currently active.
/// Note that a runtime is active if a instance of [`NetworkApplication`] exists.
///
#[must_use]
pub fn globals() -> Arc<NetworkApplicationGlobals> {
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
            NetEvents::MessageAtGateEvent(MessageAtGateEvent { gate, msg }),
            send_time,
        ));
        return;
    }

    // (0) Else handle the event inlined, for instant effects on the associated
    // channels.
    let event = MessageAtGateEvent { gate, msg };
    event.handle_with_sink(&mut ctx.events);

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
    assert!(
        restart.map_or(true, |r| r >= SimTime::now()),
        "Restart point cannot be in the past"
    );

    let mut ctx = BUF_CTX.lock();
    ctx.shutdown = Some(restart);
}

pub(crate) fn buf_set_globals(globals: Weak<NetworkApplicationGlobals>) {
    let mut ctx = BUF_CTX.lock();
    ctx.globals = Some(globals);

    // SAFTEY:
    // reseting the MOD_CTX is safe, since simulation lock is aquired.
    unsafe {
        MOD_CTX.reset(None);
    }
}

pub(crate) fn buf_process<A>(module: &ModuleRef, rt: &mut Runtime<NetworkApplication<A>>)
where
    A: EventLifecycle<NetworkApplication<A>>,
{
    let mut ctx = BUF_CTX.lock();

    // (0) Add delayed events from 'send'
    for (event, time) in ctx.events.drain(..) {
        rt.add_event(event, time);
    }

    // (1) Send loopback events from 'scheduleAt'
    for (message, time) in ctx.loopback.drain(..) {
        rt.add_event(
            NetEvents::HandleMessageEvent(HandleMessageEvent {
                module: module.clone(),
                message,
            }),
            time,
        );
    }

    // (2) Handle shutdown if indicated
    if let Some(restart) = ctx.shutdown.take() {
        // Mark the modules state
        log::debug!("Shuttind down module and restaring at {:?}", restart);
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

        // Reset the internal state
        // Note that the module is not active, so it must be manually reactivated
        module.activate();
        module.reset();
        module.deactivate(rt);

        // Reschedule wakeup
        if let Some(restart) = restart {
            rt.add_event(
                NetEvents::ModuleRestartEvent(ModuleRestartEvent {
                    module: module.clone(),
                }),
                restart,
            );
        }
    }
}
