#![allow(missing_docs)]

use super::{Globals, HandleMessageEvent, MessageExitingConnection, Sim, Watcher, WatcherValueMap};
use crate::net::gate::Connection;
use crate::net::module::{current, with_mod_ctx, MOD_CTX};
use crate::net::ModuleRestartEvent;
use crate::net::{gate::GateRef, message::Message, NetEvents};
use crate::prelude::{EventLifecycle, ModuleRef};
use crate::runtime::Runtime;
use crate::sync::Mutex;
use crate::time::SimTime;
use std::sync::{Arc, Weak};

static BUF_CTX: Mutex<BufferContext> = Mutex::new(BufferContext::new());

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
    globals: Option<Weak<Globals>>,
    // watcher
    watcher: Option<Weak<WatcherValueMap>>,
}

impl BufferContext {
    const fn new() -> Self {
        Self {
            events: Vec::new(),
            loopback: Vec::new(),
            shutdown: None,
            globals: None,
            watcher: None,
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

impl Watcher {
    pub(crate) fn current() -> Watcher {
        let ctx = BUF_CTX.lock();
        ctx.watcher
            .as_ref()
            .expect("no watcher attached to this event")
            .upgrade()
            .expect("watch already dropped: simulation shutting down")
            .watcher_for(current().path().to_string())
    }
}

pub(crate) fn buf_init(globals: Weak<Globals>, watcher: Weak<WatcherValueMap>) {
    let mut ctx = BUF_CTX.lock();
    ctx.globals = Some(globals);
    ctx.watcher = Some(watcher);

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

pub(crate) fn buf_send_at(mut msg: Message, gate: GateRef, send_time: SimTime) {
    let mut ctx = BUF_CTX.lock();
    msg.header.sender_module_id = current().id();

    crate::tracing::enter_scope(gate.owner().scope_token());

    // (0) If delayed send is active, dont skip gate_refs
    if send_time > SimTime::now() {
        ctx.events.push((
            NetEvents::MessageExitingConnection(MessageExitingConnection {
                con: Connection::new(gate),
                msg,
            }),
            send_time,
        ));
        return;
    }

    // (0) Else handle the event inlined, for instant effects on the associated
    // channels.
    let event = MessageExitingConnection {
        con: Connection::new(gate),
        msg,
    };
    event.handle_with_sink(&mut ctx.events);

    crate::tracing::enter_scope(with_mod_ctx(|ctx| ctx.scope_token));
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

pub(crate) fn buf_process<A>(module: &ModuleRef, rt: &mut Runtime<Sim<A>>)
where
    A: EventLifecycle<Sim<A>>,
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
        #[cfg(feature = "tracing")]
        tracing::debug!("Shuttind down module and restaring at {:?}", restart);
        module
            .ctx
            .active
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // drop the rt, to prevent all async activity from happening.
        #[cfg(feature = "async")]
        module.ctx.async_ext.write().rt.shutdown();

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
