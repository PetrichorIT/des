use tokio::task::yield_now;

use crate::{
    net::{
        channel::ChannelRef, gate::Connection, message::Message, module::ModuleRef,
        processing::ProcessingState, runtime::buf_process, Sim,
    },
    runtime::{EventLifecycle, EventSet, EventSink, Runtime},
    time::SimTime,
    tracing::enter_scope,
};
use std::sync::atomic::Ordering::SeqCst;

#[cfg(feature = "async")]
use crate::net::module::with_mod_ctx;

///
/// The event set for a [`NetworkApplication`].
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
pub enum NetEvents {
    MessageExitingConnection(MessageExitingConnection),
    HandleMessageEvent(HandleMessageEvent),
    ChannelUnbusyNotif(ChannelUnbusyNotif),
    ModuleRestartEvent(ModuleRestartEvent),
    #[cfg(feature = "async")]
    AsyncWakeupEvent(AsyncWakeupEvent),
}

impl<A> EventSet<Sim<A>> for NetEvents
where
    A: EventLifecycle<Sim<A>>,
{
    fn handle(self, rt: &mut Runtime<Sim<A>>) {
        match self {
            Self::MessageExitingConnection(event) => event.handle(rt),
            Self::HandleMessageEvent(event) => event.handle(rt),
            Self::ChannelUnbusyNotif(event) => event.handle(rt),
            Self::ModuleRestartEvent(event) => event.handle(rt),
            #[cfg(feature = "async")]
            Self::AsyncWakeupEvent(event) => event.handle(rt),
        }
    }
}

#[derive(Debug)]
pub struct MessageExitingConnection {
    pub(crate) con: Connection, // exiting the following connecrtion
    pub(crate) msg: Message,    // with this message
}

impl MessageExitingConnection {
    // This function executes an event with a sink not a runtime as an parameter.
    // That allows for the executing of events not handles by the runtime itself
    // aka. the calling with an abitrary event sink.
    pub(crate) fn handle_with_sink(self, sink: &mut impl EventSink<NetEvents>) {
        let mut msg = self.msg;
        msg.header.last_gate = Some(self.con.endpoint.clone());

        // The connection that was exited.
        // Current packet position: `cur.endpoint`
        let mut cur = self.con;
        while let Some(next) = cur.next_hop() {
            enter_scope(cur.endpoint.owner().scope_token());

            // Since a next gate exists log the current gate as
            // transit complete. (do this before drop check to allow for better debugging at drop)
            msg.header.last_gate = Some(next.endpoint.clone());

            // Drop message is owner is not active, but notfiy since this is an irregularity.
            if !cur.endpoint.owner().is_active() {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Gate '{}' dropped message [{}] since owner module {} is inactive",
                    cur.endpoint.name(),
                    msg.str(),
                    cur.endpoint.owner().path()
                );

                drop(msg);
                return;
            }

            // Log the current transition to the internal log stream.
            #[cfg(feature = "tracing")]
            tracing::info!(
                "Gate '{}' forwarding message [{}] to next gate delayed: {}",
                cur.endpoint.name(),
                msg.str(),
                cur.channel().is_some()
            );

            if let Some(ch) = next.channel() {
                ch.send_message(msg, next, sink);
                return;
            }

            // No channel means next hop is on the same time slot,
            // so continue.
            cur = next;
        }

        // The loop has ended. This means we are at the end of a gate chain
        // cur has not been checked for anything
        enter_scope(cur.endpoint.owner().scope_token());

        #[cfg(feature = "tracing")]
        tracing::info!(
            "Gate '{}' forwarding message [{}] to module #{}",
            cur.endpoint.name(),
            msg.str(),
            cur.endpoint.owner().id()
        );

        let module = cur.endpoint.owner();
        sink.add(
            NetEvents::HandleMessageEvent(HandleMessageEvent {
                module,
                message: msg,
            }),
            SimTime::now(),
        );
    }
}

impl MessageExitingConnection {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        self.handle_with_sink(rt);
    }
}

#[derive(Debug)]
pub struct HandleMessageEvent {
    pub(crate) module: ModuleRef,
    pub(crate) message: Message,
}

impl HandleMessageEvent {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        enter_scope(self.module.scope_token());

        let mut message = self.message;
        message.header.receiver_module_id = self.module.ctx.id;

        #[cfg(feature = "tracing")]
        tracing::info!("Handling message {:?}", message.str());

        let module = &self.module;

        module.activate();
        module.handle_message(message);
        module.deactivate(rt);

        buf_process(module, rt);
    }
}

#[derive(Debug)]
pub struct ModuleRestartEvent {
    pub(crate) module: ModuleRef,
}

impl ModuleRestartEvent {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        enter_scope(self.module.scope_token());

        #[cfg(feature = "tracing")]
        tracing::info!("ModuleRestartEvent");

        let module = &self.module;
        module.activate();
        module.module_restart();
        module.deactivate(rt);

        buf_process(module, rt);
    }
}

#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncWakeupEvent {
    pub(crate) module: ModuleRef,
}

#[cfg(feature = "async")]
impl AsyncWakeupEvent {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        enter_scope(self.module.scope_token());

        #[cfg(feature = "tracing")]
        tracing::info!("async wakeup");

        let module = &self.module;
        module.activate();
        module.async_wakeup();
        module.deactivate(rt);

        buf_process(module, rt);
    }
}

#[derive(Debug)]
pub struct ChannelUnbusyNotif {
    pub(crate) channel: ChannelRef,
}

impl ChannelUnbusyNotif {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        self.channel.unbusy(rt);
    }
}

#[cfg(not(feature = "async"))]
fn with_harness<R>(f: impl FnOnce() -> R) -> R {
    f()
}

#[cfg(feature = "async")]
fn with_harness<R>(f: impl FnOnce() -> R) -> R {
    let Some((rt, task_set)) = with_mod_ctx(|ctx| ctx.async_ext.write().rt.current()) else {
        panic!("WHERE MY RT");
    };

    task_set.block_on(&rt, async move {
        let ret = f();
        tokio::task::yield_now().await;
        ret
    })
}

impl ModuleRef {
    pub(crate) fn reset(&self) {
        let mut brw = self.processing.borrow_mut();

        #[cfg(feature = "async")]
        with_mod_ctx(|ctx| ctx.async_ext.write().reset());

        with_harness(move || brw.handler.reset());
    }

    #[cfg(feature = "async")]
    pub(crate) fn async_wakeup(&self) {
        if self.ctx.active.load(SeqCst) {
            self.processing.borrow_mut().incoming_upstream(None);
            with_harness(|| {});
            self.processing.borrow_mut().incoming_downstream();
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!("Ignoring message since module is inactive");
        }
    }

    pub(crate) fn module_restart(&self) {
        #[cfg(feature = "tracing")]
        tracing::debug!("Restarting module");
        // restart the module itself.
        self.ctx.active.store(true, SeqCst);

        // Do sim start procedure
        let stages = self.num_sim_start_stages();
        for stage in 0..stages {
            self.at_sim_start(stage);
        }
    }

    pub(crate) fn handle_message(&self, msg: Message) {
        if self.ctx.active.load(SeqCst) {
            let mut processing = self.processing.borrow_mut();

            // Upstream
            let msg = processing.incoming_upstream(Some(msg));

            // Peek
            processing.state = ProcessingState::Peek;
            if let Some(msg) = msg {
                with_harness(|| {
                    let msg = msg;
                    processing.handler.handle_message(msg)
                });
            } else {
                with_harness(|| {})
            }

            // Downstream
            processing.incoming_downstream();
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!("Ignoring message since module is inactive");
        }
    }

    pub(crate) fn at_sim_start(&self, stage: usize) {
        let mut processing = self.processing.borrow_mut();

        processing.incoming_upstream(None);
        with_harness(|| processing.handler.at_sim_start(stage));
        processing.incoming_downstream();
    }

    pub(crate) fn num_sim_start_stages(&self) -> usize {
        // No harness since this method bust be called before startin initalization to check the number of loops
        self.processing.borrow().handler.num_sim_start_stages()
    }

    pub(crate) fn at_sim_end(&self) {
        let mut processing = self.processing.borrow_mut();

        processing.incoming_upstream(None);
        with_harness(|| processing.handler.at_sim_end());
        #[cfg(feature = "async")]
        {
            let Some((rt, task_set)) = with_mod_ctx(|ctx| ctx.async_ext.write().rt.current())
            else {
                panic!("WHERE MY RT");
            };
            let mut joins = Vec::new();
            with_mod_ctx(|ctx| {
                std::mem::swap(&mut ctx.async_ext.write().require_joins, &mut joins)
            });

            let _guard = rt.enter();
            task_set.block_on(&rt, yield_now());

            for join in joins {
                if !join.is_finished() {
                    panic!("could not join task: not yet finished")
                }

                let join_result = rt.block_on(join);
                if let Err(e) = join_result {
                    panic!("could not join task: {e}")
                }
            }
        }
        processing.incoming_downstream();
    }
}
