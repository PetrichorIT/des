use crate::{
    net::{
        Error, Sim,
        channel::{ChannelRef, SendContext, SendError},
        gate::Connection,
        message::Message,
        module::ModuleRef,
        runtime::buf_process,
        schedule_event,
    },
    prelude::RuntimeError,
    runtime::{Event, EventLifecycle, EventSink, Runtime},
    time::SimTime,
    tracing::{enter_scope, leave_scope},
};
use std::{
    any::Any,
    fmt::Debug,
    sync::atomic::Ordering::{self, SeqCst},
};

use super::Harness;

#[cfg(feature = "async")]
use crate::net::processing::TokioRuntime;

///
/// The event set for a [`Sim`].
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
pub enum NetEvents {
    /// A message exiting a connection, implemented by a channel
    MessageExitingConnection(MessageExitingConnection),
    /// A message arrival at the end of a gate chain.
    HandleMessageEvent(HandleMessageEvent),
    /// A notification for channels.
    ChannelUnbusyNotif(ChannelUnbusyNotif),
    /// A delayed `at_sim_start` event for a spawned module.
    AtSimStartEvent(AtSimStartEvent),
    /// A notification that a module should now be restarted
    ModuleShutdownEvent(ModuleShutdownEvent),
    /// A notification that a module should now be restarted
    ModuleRestartEvent(ModuleRestartEvent),
    #[cfg(feature = "async")]
    /// A async wakeup
    AsyncWakeupEvent(AsyncWakeupEvent),
}

impl<A> Event<Sim<A>> for NetEvents
where
    A: EventLifecycle<Sim<A>>,
{
    fn handle(self, rt: &mut Runtime<Sim<A>>) {
        match self {
            Self::MessageExitingConnection(event) => event.handle(rt),
            Self::HandleMessageEvent(event) => event.handle(rt),
            Self::ChannelUnbusyNotif(event) => event.handle(rt),
            Self::AtSimStartEvent(event) => event.handle(rt),
            Self::ModuleShutdownEvent(event) => event.handle(rt),
            Self::ModuleRestartEvent(event) => event.handle(rt),
            #[cfg(feature = "async")]
            Self::AsyncWakeupEvent(event) => event.handle(rt),
        }
    }
}

/// A message exiting a connection, implemented by a channel.
#[derive(Debug)]
pub struct MessageExitingConnection {
    /// The connection that was now traversed.
    pub con: Connection,
    /// The message.
    pub msg: Message,
}

impl MessageExitingConnection {
    // This function executes an event with a sink not a runtime as an parameter.
    // That allows for the executing of events not handles by the runtime itself
    // aka. the calling with an abitrary event sink.
    pub(crate) fn handle_with_sink(
        self,
        sink: &mut impl EventSink<NetEvents>,
    ) -> Result<(), SendError> {
        let mut msg = self.msg;
        msg.header.last_gate = Some(self.con.endpoint.clone());

        // The connection that was exited.
        // Current packet position: `cur.endpoint`
        let mut cur = self.con;
        while let Some(next) = cur.next_hop() {
            enter_scope(cur.endpoint.owner().scope_token());

            let cur_endpoint = cur.endpoint.clone();

            // Since a next gate exists log the current gate as
            // transit complete. (do this before drop check to allow for better debugging at drop)
            msg.header.last_gate = Some(next.endpoint.clone());

            // Drop message is owner is not active, but notfiy since this is an irregularity.
            if !cur.endpoint.owner().is_active() {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Gate '{}' dropped message [{}] since owner module {} is inactive",
                    cur.endpoint.name(),
                    msg,
                    cur.endpoint.owner().path()
                );

                return Err(SendError {
                    msg,
                    reason: "Endpoint module is inactive".into(),
                });
            }

            // Log the current transition to the internal log stream.
            #[cfg(feature = "tracing")]
            tracing::info!(
                "Gate '{}' forwarding message [{}] to next gate delayed: {}",
                cur.endpoint.name(),
                msg,
                cur.channel().is_some()
            );

            if let Some(ch) = next.channel() {
                let ctx = SendContext {
                    sink,
                    handle: ch.clone(),
                };
                return ch.channel.try_write().expect("failed lock").send(
                    cur_endpoint,
                    msg,
                    next,
                    ctx,
                );
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
            msg,
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

        Ok(())
    }
}

impl MessageExitingConnection {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        let result = self.handle_with_sink(rt);
        if let Err(err) = result {
            tracing::error!("message {} failed to be send: {}", err.msg, err.reason);
        }
    }
}

/// A message entering a module, by existing a gate-chain or being self-scheduled.
#[derive(Debug)]
pub struct HandleMessageEvent {
    /// The module that the message is arriving at..
    pub module: ModuleRef,
    /// The message being handled.
    pub message: Message,
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
        tracing::info!("Handling message {:?}", message);

        let module = &self.module;

        let _ = module.activate();
        rt.app.error.extend(module.handle_message(message).err());
        module.deactivate();

        buf_process(module, rt);
    }
}

/// A delayed `at_sim_start` event for a spawned module.
#[derive(Debug)]
pub struct AtSimStartEvent {
    /// The module that is being spawned.
    pub modules: Vec<ModuleRef>,
}

impl AtSimStartEvent {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        let max_stage = self
            .modules
            .iter()
            .fold(1, |acc, module| acc.max(module.num_sim_start_stages()));

        for stage in 0..max_stage {
            // Direct indexing since rt must be borrowed mutably in handle_buffers.
            for module in &self.modules {
                // Use cloned handles to appease the brwchk
                if stage < module.num_sim_start_stages() {
                    let _ = module.activate();

                    #[cfg(feature = "tracing")]
                    tracing::info!("Calling at_sim_start({}).", stage);

                    rt.app.error.extend(module.at_sim_start(stage).err());
                    module.deactivate();

                    buf_process(module, rt);
                }
            }
        }

        leave_scope();
    }
}

/// A notification that a module should now be shutdown.
#[derive(Debug)]
pub struct ModuleShutdownEvent {
    /// The module that is being shutdown.
    pub module: ModuleRef,
    /// The time at which the module should be restarted, if any.
    pub restart_at: Option<SimTime>,
}

impl ModuleShutdownEvent {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        enter_scope(self.module.scope_token());

        #[cfg(feature = "tracing")]
        tracing::info!("ModuleShutdownEvent");

        let module = &self.module;
        let _ = module.activate();
        rt.app
            .error
            .extend(module.module_shutdown(self.restart_at).err());
        module.deactivate();

        buf_process(module, rt);
    }
}

/// A notification to restart a module.
#[derive(Debug)]
pub struct ModuleRestartEvent {
    /// The module that is being restarted.
    pub module: ModuleRef,
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
        let _ = module.activate();
        rt.app.error.extend(module.module_restart().err());
        module.deactivate();

        buf_process(module, rt);
    }
}

/// An async wakeup to indicate to tokio that some progress can now be made.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncWakeupEvent {
    /// The module
    pub module: ModuleRef,
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
        let _ = module.activate();
        rt.app.error.extend(module.async_wakeup().err());
        module.deactivate();

        buf_process(module, rt);
    }
}

/// A notification for a channel, that some timer has expired. Usually used
/// to indicate that the busy phase (aka the sending phase) has completed.
#[derive(Debug)]
pub struct ChannelUnbusyNotif {
    /// The affected channel
    pub channel: ChannelRef,
    /// Additional information about the wakeup event
    pub info: Box<dyn Any + Send>,
}

impl ChannelUnbusyNotif {
    fn handle<A>(self, rt: &mut Runtime<Sim<A>>)
    where
        A: EventLifecycle<Sim<A>>,
    {
        let handle = self.channel.clone();
        self.channel
            .channel
            .try_write()
            .expect("failed to get lock")
            .unbusy_notify(self.info, SendContext { sink: rt, handle });
    }
}

impl ModuleRef {
    /// Resetting a module state as port of a reboot or shutdown sequence
    ///
    /// This function must do the following things:
    /// - reset the modules internal state (this may be `self = Self::new()`), but maybe some
    ///   persistent state should be preserved.
    /// - reset the proc-chain
    pub(crate) fn reset(&self) -> Result<(), Error> {
        let mut brw = self.processing.borrow_mut();

        // FIXME: the reset of the proc-chain would be easiers if we could
        // rebuild the chain from scratch. However we would need the base_stack for
        // that to work, we dont have it here, since its attached to the Builder instance of `Sim`
        //
        // TODO: capture Sim<A> -> easy
        // TODO: store the used base stack somehow or recreate it?
        // TODO: then call stack() on the reset module itself
        #[cfg(feature = "async")]
        brw.downcast_element_mut::<TokioRuntime>()
            .map(TokioRuntime::reset);

        // Reset does not capture any proc-elements -> correct ?
        Harness::new(&self.ctx)
            .exec(move || brw.handler.reset())
            .pass()?;
        Ok(())
    }

    #[cfg(feature = "async")]
    pub(crate) fn async_wakeup(&self) -> Result<(), Error> {
        if self.ctx.active.load(SeqCst) {
            self.processing
                .borrow_mut()
                .process_with(None, |_, _| Harness::new(&self.ctx).exec(|| {}).catch())?;
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!("Ignoring message since module is inactive");
        }
        Ok(())
    }

    pub(crate) fn module_shutdown(&self, restart_at: Option<SimTime>) -> Result<(), Error> {
        if !self.active.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Mark the modules state
        #[cfg(feature = "tracing")]
        tracing::debug!("Shuttind down module and restaring at {:?}", restart_at);
        self.ctx.active.store(false, SeqCst);

        // drop the rt, to prevent all async activity from happening.
        #[cfg(feature = "async")]
        self.processing
            .borrow_mut()
            .downcast_element_mut::<TokioRuntime>()
            .map(TokioRuntime::shutdown);

        // Reset the internal state
        // Note that the module is not active, so it must be manually reactivated
        let res = self.reset();

        // Reschedule wakeup
        if let Some(restart_at) = restart_at {
            schedule_event(
                NetEvents::ModuleRestartEvent(ModuleRestartEvent {
                    module: self.clone(),
                }),
                restart_at,
            );
        }
        res
    }

    pub(crate) fn module_restart(&self) -> Result<(), Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!("Restarting module");
        // restart the module itself.
        self.ctx.active.store(true, SeqCst);

        // Do sim start procedure
        let stages = self.num_sim_start_stages();
        for stage in 0..stages {
            self.at_sim_start(stage)?;
        }
        Ok(())
    }

    pub(crate) fn handle_message(&self, msg: Message) -> Result<(), Error> {
        if self.ctx.active.load(SeqCst) {
            self.processing
                .borrow_mut()
                .process_with(Some(msg), |handler, msg| {
                    if let Some(msg) = msg {
                        Harness::new(&self.ctx)
                            .exec(|| {
                                let msg = msg;
                                handler.handle_message(msg);
                            })
                            .catch()
                    } else {
                        Harness::new(&self.ctx).exec(|| {}).catch()
                    }
                })?;
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!("Ignoring message since module is inactive");
        }
        Ok(())
    }

    pub(crate) fn at_sim_start(&self, stage: usize) -> Result<(), Error> {
        self.processing
            .borrow_mut()
            .process_with(None, |handler, _| {
                Harness::new(&self.ctx)
                    .exec(|| handler.at_sim_start(stage))
                    .catch()
            })?;
        Ok(())
    }

    pub(crate) fn num_sim_start_stages(&self) -> usize {
        // No harness since this method bust be called before startin initalization to check the number of loops
        // Bypass the CTX variables, this should be safe maybe
        self.processing.borrow().handler.num_sim_start_stages()
    }

    pub(crate) fn at_sim_end(&self) -> Result<(), RuntimeError> {
        #[allow(unused_mut)]
        let mut result = self
            .processing
            .borrow_mut()
            .process_with(None, |handler, _| {
                let mut result = Ok(());

                Harness::new(&self.ctx)
                    .exec(|| result = handler.at_sim_end())
                    .catch()?;

                result
            });

        let mut processing = self.processing.borrow_mut();
        processing.process_with(None, |_, _| {});

        #[cfg(feature = "async")]
        let Some(tokio) = processing.downcast_element_mut::<TokioRuntime>() else {
            return result;
        };

        #[cfg(feature = "async")]
        if let Err(other) = tokio.at_sim_end() {
            result = Err(other);
        }

        result
    }
}
