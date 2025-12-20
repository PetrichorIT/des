#![allow(missing_docs)]

use super::{Globals, HandleMessageEvent, MessageExitingConnection, Sim};
use crate::{
    net::{
        channel::SendError,
        gate::{Connection, GateRef},
        message::Message,
        module::{MOD_CTX, current},
        runtime::NetEvents,
    },
    prelude::{EventLifecycle, ModuleRef, RuntimeError},
    runtime::{LikeRuntimeError, Runtime},
    sync::Mutex,
    time::SimTime,
};
use std::mem;
use std::sync::{Arc, Weak};

static BUF_CTX: Mutex<BufferContext> = Mutex::new(BufferContext::new());

struct BufferContext {
    // All new events that will be scheduled
    events: Vec<(NetEvents, SimTime)>,
    // globals
    globals: Option<Weak<Globals>>,
    // errors
    error: Vec<Box<dyn LikeRuntimeError>>,
    // failure
    failure: Result<(), RuntimeError>,
}

impl BufferContext {
    const fn new() -> Self {
        Self {
            events: Vec::new(),
            globals: None,
            error: Vec::new(),
            failure: Ok(()),
        }
    }
}

unsafe impl Send for BufferContext {}
unsafe impl Sync for BufferContext {}

impl Globals {
    pub(crate) fn current() -> Arc<Self> {
        let ctx = BUF_CTX.lock();
        ctx.globals
            .as_ref()
            .expect("no globals attached to this event")
            .upgrade()
            .expect("globals allready dropped: simulation shutting down")
    }
}

pub(crate) fn buf_init(globals: Weak<Globals>) {
    let mut ctx = BUF_CTX.lock();
    ctx.globals = Some(globals);

    // TODO: remove ?
    // SAFTEY:
    // reseting the MOD_CTX is safe, since simulation lock is aquired.
    unsafe {
        MOD_CTX.reset(None);
    }
}

pub(crate) fn buf_drop() {
    let mut ctx = BUF_CTX.lock();
    *ctx = BufferContext::new();
}

pub(crate) fn buf_send_at(
    mut msg: Message,
    gate: GateRef,
    send_time: SimTime,
) -> Result<(), SendError> {
    let mut ctx = BUF_CTX.lock();
    msg.header.sender_module_id = current().id();

    // (0) If delayed send is active, dont skip gate_refs
    if send_time > SimTime::now() {
        ctx.events.push((
            NetEvents::MessageExitingConnection(MessageExitingConnection {
                con: Connection::new(gate),
                msg,
            }),
            send_time,
        ));
        return Ok(());
    }

    // (0) Else handle the event inlined, for instant effects on the associated
    // channels.
    let event = MessageExitingConnection {
        con: Connection::new(gate),
        msg,
    };
    event.handle_with_sink(&mut ctx.events)
}

pub(crate) fn buf_schedule_at(msg: Message, arrival_time: SimTime) {
    // continue to delay the delivery of event, since non other components are
    // used, and we dont block any channels. additionally this ensures that
    // timeouts are allways ordered later than packets, which is good
    let mut ctx = BUF_CTX.lock();
    ctx.events.push((
        NetEvents::HandleMessageEvent(HandleMessageEvent {
            module: current().me(),
            message: msg,
        }),
        arrival_time,
    ));
}

pub(crate) fn buf_schedule_event(event: NetEvents, time: SimTime) {
    let mut ctx = BUF_CTX.lock();
    ctx.events.push((event, time));
}

pub(crate) fn buf_process<A>(
    _module: &ModuleRef,
    rt: &mut Runtime<Sim<A>>,
) -> Result<(), RuntimeError>
where
    A: EventLifecycle<Sim<A>>,
{
    let mut ctx = BUF_CTX.lock();

    // (0) Add delayed events from 'send'
    for (event, time) in ctx.events.drain(..) {
        rt.add_event(event, time);
    }

    // (1) Pull collected failures from CTX
    rt.app.error.extend(ctx.error.drain(..));

    let mut swaped_out = Ok(());
    mem::swap(&mut swaped_out, &mut ctx.failure);
    swaped_out
}

pub(crate) fn buf_report(e: impl LikeRuntimeError) {
    let mut ctx = BUF_CTX.lock();
    ctx.error.push(Box::new(e));
}

pub(crate) fn buf_fail(e: RuntimeError) {
    let mut ctx = BUF_CTX.lock();
    match &mut ctx.failure {
        Ok(()) => ctx.failure = Err(e),
        Err(err) => err.merge(e),
    }
}
