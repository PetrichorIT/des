use crate::{
    net::{
        Error, ErrorKind, Failure, Sim,
        channel::{ChannelRef, SendContext, SendError},
        gate::Connection,
        message::{Body, Message},
        module::{
            ModuleContext, ModuleRef, SIGNAL_MODULE_PANICED, SIGNAL_SIM_START_DONE, Signal, State,
            emit,
        },
        runtime::{EventExecutionContext, cfg::SimLifecycle},
        schedule_event,
    },
    runtime::{Event, EventSink, Runtime},
    time::SimTime,
};
use std::{
    any::Any,
    fmt::Debug,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
    task::Waker,
};

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
    /// A signal that appeared on a module, to be handled by other nodes.
    SignalEvent(SignalEvent),
    /// A notification that a module should now be restarted
    ModuleShutdownEvent(ModuleShutdownEvent),
    /// A notification that a module should now be restarted
    ModuleRestartEvent(ModuleRestartEvent),
    #[cfg(feature = "async")]
    /// A async wakeup
    AsyncWakeupEvent(AsyncWakeupEvent),
}

impl<A: SimLifecycle> Event<Sim<A>> for NetEvents {
    fn handle(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        match self {
            Self::MessageExitingConnection(event) => event.handle(rt),
            Self::HandleMessageEvent(event) => event.handle(rt),
            Self::ChannelUnbusyNotif(event) => event.handle(rt),
            Self::AtSimStartEvent(event) => event.handle(rt),
            Self::SignalEvent(event) => event.handle(rt),
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
    #[allow(clippy::unnecessary_wraps)]
    fn handle<A: SimLifecycle>(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        let result = self.handle_with_sink(rt);
        if let Err(err) = result {
            tracing::error!("message {} failed to be send: {}", err.msg, err.reason);
        }
        Ok(())
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
    fn handle<A: SimLifecycle>(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        let message = self.message;
        let module = &self.module;

        #[cfg(feature = "tracing")]
        tracing::info!("Handling message {:?}", message);

        let ctx = EventExecutionContext::default();

        module.activate_with(Some(ctx.clone()));
        rt.app.error.extend(module.handle_message(message).err());
        module.deactivate();

        ctx.finish(rt)
    }
}

/// A delayed `at_sim_start` event for a spawned module.
#[derive(Debug)]
pub struct AtSimStartEvent {
    /// The module that is being spawned.
    pub modules: Vec<ModuleRef>,
}

impl AtSimStartEvent {
    fn handle<A: SimLifecycle>(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        let max_stage = self
            .modules
            .iter()
            .fold(1, |acc, module| acc.max(module.num_sim_start_stages()));

        for stage in 0..max_stage {
            // Direct indexing since rt must be borrowed mutably in handle_buffers.
            for module in &self.modules {
                // Use cloned handles to appease the brwchk
                if stage < module.num_sim_start_stages() {
                    let ctx = EventExecutionContext::default();

                    module.activate_with(Some(ctx.clone()));
                    #[cfg(feature = "tracing")]
                    tracing::info!("Calling at_sim_start({}).", stage);
                    rt.app.error.extend(module.at_sim_start(stage).err());
                    module.deactivate();

                    ctx.finish(rt)?;
                }
            }
        }

        Ok(())
    }
}

/// A signal event that is emitted when a signal is received.
#[derive(Debug)]
pub struct SignalEvent {
    /// The signal.
    pub signal: Signal,
    /// The set of all subscribers to the signal (keeping them as a set reduces event count).
    pub subscribers: Vec<ModuleRef>,
}

impl SignalEvent {
    fn handle<A: SimLifecycle>(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        for subscriber in &self.subscribers {
            let ctx = EventExecutionContext::default();

            subscriber.activate_with(Some(ctx.clone()));
            rt.app
                .error
                .extend(subscriber.handle_signal(self.signal.clone()).err());
            subscriber.deactivate();

            ctx.finish(rt)?;
        }

        Ok(())
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
    fn handle<A: SimLifecycle>(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        #[cfg(feature = "tracing")]
        tracing::info!("ModuleShutdownEvent");

        let module = &self.module;
        let ctx = EventExecutionContext::default();

        module.activate_with(Some(ctx.clone()));
        rt.app
            .error
            .extend(module.module_shutdown(self.restart_at).err());
        module.deactivate();

        ctx.finish(rt)
    }
}

/// A notification to restart a module.
#[derive(Debug)]
pub struct ModuleRestartEvent {
    /// The module that is being restarted.
    pub module: ModuleRef,
}

impl ModuleRestartEvent {
    fn handle<A: SimLifecycle>(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        #[cfg(feature = "tracing")]
        tracing::info!("ModuleRestartEvent");

        let module = &self.module;
        let ctx = EventExecutionContext::default();

        module.activate_with(Some(ctx.clone()));
        rt.app.error.extend(module.module_restart().err());
        module.deactivate();

        ctx.finish(rt)
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
    fn handle<A: SimLifecycle>(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        #[cfg(feature = "tracing")]
        tracing::info!("async wakeup");

        let module = &self.module;
        let ctx = EventExecutionContext::default();

        module.activate_with(Some(ctx.clone()));
        rt.app.error.extend(module.async_wakeup().err());
        module.deactivate();

        ctx.finish(rt)
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
    #[allow(clippy::unnecessary_wraps)]
    fn handle<A: SimLifecycle>(self, rt: &mut Runtime<Sim<A>>) -> Result<(), Failure> {
        let handle = self.channel.clone();
        self.channel
            .channel
            .try_write()
            .expect("failed to get lock")
            .unbusy_notify(self.info, SendContext { sink: rt, handle });

        Ok(())
    }
}

impl ModuleRef {
    pub(crate) fn activate_with(
        &self,
        exec: Option<EventExecutionContext>,
    ) -> Option<Arc<ModuleContext>> {
        self.ctx.set_execution_context(exec);
        let prev = ModuleContext::place(Arc::clone(&self.ctx));
        #[cfg(debug_assertions)]
        if let Some(prev) = &prev {
            eprintln!("pushed-off ctx from {}", prev.path());
        }
        prev
    }

    pub(crate) fn deactivate(&self) {
        let _ = ModuleContext::take();
        self.ctx.set_execution_context(None);
    }

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
        if matches!(self.ctx.state.get(), State::Running) {
            self.processing
                .borrow_mut()
                .process_with(None, |_, _| Harness::new(&self.ctx).exec(|| {}).catch())?;
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!("Ignoring message since module is inactive");
        }
        Ok(())
    }

    pub(crate) fn handle_signal(&self, signal: Signal) -> Result<(), Error> {
        // Custom signal handlers
        if signal.code == SIGNAL_SIM_START_DONE {
            self.state_change_wakers
                .write()
                .drain(..)
                .for_each(Waker::wake);
        }

        self.processing
            .borrow_mut()
            .process_with(None, move |handler, _| {
                Harness::new(&self.ctx)
                    .exec(|| handler.handle_signal(signal))
                    .catch()
            })?;
        Ok(())
    }

    pub(crate) fn module_shutdown(&self, restart_at: Option<SimTime>) -> Result<(), Error> {
        if matches!(self.ctx.state.get(), State::Shutdown) {
            return Ok(());
        }

        // Mark the modules state
        #[cfg(feature = "tracing")]
        tracing::debug!("Shuttind down module and restaring at {:?}", restart_at);
        self.ctx.state.set(State::Shutdown);

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
        self.ctx.state.set(State::Initialized);

        // Do sim start procedure
        let stages = self.num_sim_start_stages();
        for stage in 0..stages {
            self.at_sim_start(stage)?;
        }
        Ok(())
    }

    pub(crate) fn handle_message(&self, msg: Message) -> Result<(), Error> {
        if let State::Running = self.ctx.state.get() {
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
        let mut max = 0;
        self.processing
            .borrow_mut()
            .process_with(None, |handler, _| {
                Harness::new(&self.ctx)
                    .exec(|| {
                        max = handler.num_sim_start_stages();
                        handler.at_sim_start(stage);
                    })
                    .catch()
            })?;

        if stage + 1 == max {
            self.ctx.state.set(State::Running);
            schedule_event(
                NetEvents::SignalEvent(SignalEvent {
                    signal: Signal {
                        source: self.clone(),
                        code: SIGNAL_SIM_START_DONE,
                        body: Body::empty(),
                    },
                    subscribers: vec![self.clone()],
                }),
                SimTime::now(),
            );
        }

        Ok(())
    }

    pub(crate) fn num_sim_start_stages(&self) -> usize {
        // No harness since this method bust be called before startin initalization to check the number of loops
        // Bypass the CTX variables, this should be safe maybe
        self.processing.borrow().handler.num_sim_start_stages()
    }

    pub(crate) fn at_sim_end(&self) -> Result<(), Failure> {
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
        if let Some(tokio) = processing.downcast_element_mut::<TokioRuntime>()
            && let Err(other) = tokio.at_sim_end()
        {
            match result {
                Ok(()) => return Err(Failure::from(other)),
                Err(e) => {
                    return {
                        let mut f = Failure::from(e);
                        f.extend(other);
                        Err(f)
                    };
                }
            }
        }

        self.export_statistics_report()?;

        result.map_err(Failure::from)
    }
}

#[must_use]
pub(super) struct Harness<'a> {
    ctx: &'a ModuleContext,
    unwind: Option<Box<dyn Any + Send + 'static>>,
}

impl<'a> Harness<'a> {
    pub(super) fn new(ctx: &'a ModuleContext) -> Self {
        Harness { ctx, unwind: None }
    }

    pub(super) fn exec(mut self, f: impl FnOnce()) -> Self {
        self.unwind = catch_unwind(AssertUnwindSafe(f)).err();
        self
    }

    pub(super) fn catch(self) -> Result<(), Error> {
        if let Some(unwind) = self.unwind {
            let bh = self.ctx.unwind_behaviour();

            self.ctx.state.set(State::Shutdown);

            emit(SIGNAL_MODULE_PANICED, Body::empty());

            if !bh.on_panic_catch {
                return Err(Error::new(self.ctx.path(), ErrorKind::ModulePanic(unwind)));
            }

            if bh.on_panic_restart {
                schedule_event(
                    NetEvents::ModuleRestartEvent(ModuleRestartEvent {
                        module: self.ctx.me(),
                    }),
                    SimTime::now(),
                );
            }

            if bh.on_panic_drop_submodules {
                // TODO: impl drop submodules
            }
        }
        Ok(())
    }

    pub(super) fn pass(self) -> Result<(), Error> {
        if let Some(unwind) = self.unwind {
            return Err(Error::new(self.ctx.path(), ErrorKind::ModulePanic(unwind)));
        }
        Ok(())
    }
}
