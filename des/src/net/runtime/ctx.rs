#![allow(missing_docs)]

use super::{HandleMessageEvent, NetworkRuntime, NetworkRuntimeGlobals};
use crate::net::{gate::GateRef, message::Message, MessageAtGateEvent, NetEvents};
use crate::prelude::{GateServiceType, ModuleRef};
use crate::runtime::Runtime;
use crate::time::SimTime;
use std::cell::RefCell;
use std::sync::{Arc, Weak};

thread_local! {
     static BUF_CTX: RefCell<BufferContext> = const {
        RefCell::new(BufferContext {
            output: Vec::new(),
            loopback: Vec::new(),
            shutdown: None,
            globals: None,
        })
    }
}

struct BufferContext {
    // (Message, OutGate, SendTime)
    output: Vec<(Message, GateRef, SimTime)>,
    // (Message, SendTime)
    loopback: Vec<(Message, SimTime)>,
    // shudown,
    #[allow(clippy::option_option)]
    shutdown: Option<Option<SimTime>>,
    // globals
    globals: Option<Weak<NetworkRuntimeGlobals>>,
}

///
/// Returns the globals of the runtime.
///
#[must_use]
pub fn globals() -> Arc<NetworkRuntimeGlobals> {
    BUF_CTX.with(|ctx| {
        ctx.borrow()
            .globals
            .as_ref()
            .unwrap()
            .upgrade()
            .expect("No runtime globals attached")
    })
}

pub(crate) fn buf_send_at(msg: Message, gate: GateRef, send_time: SimTime) {
    BUF_CTX.with(|ctx| ctx.borrow_mut().output.push((msg, gate, send_time)));
}
pub(crate) fn buf_schedule_at(msg: Message, arrival_time: SimTime) {
    BUF_CTX.with(|ctx| ctx.borrow_mut().loopback.push((msg, arrival_time)));
}
pub(crate) fn buf_schedule_shutdown(restart: Option<SimTime>) {
    BUF_CTX.with(|ctx| ctx.borrow_mut().shutdown = Some(restart));
}

pub(crate) fn buf_set_globals(globals: Weak<NetworkRuntimeGlobals>) {
    BUF_CTX.with(|ctx| ctx.borrow_mut().globals = Some(globals));
}

pub(crate) fn buf_process<A>(module: &ModuleRef, rt: &mut Runtime<NetworkRuntime<A>>) {
    let self_id = module.ctx.id;

    BUF_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();

        // Send gate events from the 'send' method calls
        for (mut message, gate, time) in ctx.output.drain(..) {
            assert!(
                gate.service_type() != GateServiceType::Input,
                "To send messages onto a gate it must have service type of 'Output' or 'Undefined'"
            );
            message.header.sender_module_id = self_id;
            rt.add_event(
                NetEvents::MessageAtGateEvent(MessageAtGateEvent { gate, message }),
                time,
            );
        }

        // Send loopback events from 'scheduleAt'
        for (message, time) in ctx.loopback.drain(..) {
            rt.add_event(
                NetEvents::HandleMessageEvent(HandleMessageEvent {
                    module: module.clone(),
                    message,
                }),
                time,
            );
        }

        // MARKER: shutdown
        if let Some(rest) = ctx.shutdown.take() {
            log::debug!("Shuttind down module and restaring at {:?}", rest);
            use crate::net::message::TYP_RESTART;

            module
                .ctx
                .active
                .store(false, std::sync::atomic::Ordering::SeqCst);

            // drop the rt, to prevent all async activity from happening.
            #[cfg(feature = "async")]
            #[cfg(not(feature = "async-sharedrt"))]
            drop(module.ctx.async_ext.borrow_mut().rt.take());

            // drop all hooks to ensure all messages reach the async impl
            module.ctx.hooks.borrow_mut().clear();

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

        // TODO: Reintroduce
        // if !rt.app.globals.parameters.updates.borrow().is_empty() {
        //     for update in rt.app.globals.parameters.updates.borrow_mut().drain(..) {
        //         rt.app
        //             .module(|m| m.name() == update)
        //             .unwrap()
        //             .handle_par_change();
        //     }
        // }
    });
}
