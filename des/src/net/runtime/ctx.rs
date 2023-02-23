#![allow(missing_docs)]

use super::{HandleMessageEvent, NetworkRuntime, NetworkRuntimeGlobals};
use crate::net::module::{MOD_CTX, SETUP_FN};
use crate::net::{gate::GateRef, message::Message, MessageAtGateEvent, NetEvents};
use crate::prelude::{EventLifecycle, GateServiceType, ModuleRef};
use crate::runtime::Runtime;
use crate::sync::Mutex;
use crate::time::SimTime;
use std::sync::{Arc, Weak};

static BUF_CTX: Mutex<BufferContext> = Mutex::new(BufferContext {
    output: Vec::new(),
    loopback: Vec::new(),
    shutdown: None,
    globals: None,
});

type OutputBuffer = Vec<(Message, GateRef, SimTime)>;
type LoopbackBuffer = Vec<(Message, SimTime)>;

struct BufferContext {
    // (Message, OutGate, SendTime)
    output: OutputBuffer,
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

pub(crate) fn buf_send_at(msg: Message, gate: GateRef, send_time: SimTime) {
    let mut ctx = BUF_CTX.lock();
    ctx.output.push((msg, gate, send_time));
}
pub(crate) fn buf_schedule_at(msg: Message, arrival_time: SimTime) {
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
    let self_id = module.ctx.id;
    let mut ctx = BUF_CTX.lock();

    // Send gate events from the 'send' method calls
    for (mut message, gate, time) in ctx.output.drain(..) {
        assert!(
            gate.service_type() != GateServiceType::Input,
            "To send messages onto a gate it must have service type of 'Output' or 'Undefined'"
        );
        // std::thread::sleep(Duration::from_millis(100));
        // let secs = time.as_secs();
        // println!("adding message: {} at {}", message.str(), time);
        message.header.sender_module_id = self_id;
        rt.add_event(
            NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                gate,
                message: Box::new(message),
            }),
            time,
        );
    }

    // Send loopback events from 'scheduleAt'
    for (message, time) in ctx.loopback.drain(..) {
        rt.add_event(
            NetEvents::HandleMessageEvent(HandleMessageEvent {
                module: module.clone(),
                message: Box::new(message),
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
                    message: Box::new(Message::new().typ(TYP_RESTART).build()),
                }),
                rest,
            );
        }
    }
}
