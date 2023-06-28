use std::sync::atomic::Ordering::SeqCst;

use crate::{
    net::{
        gate::GateRef,
        gate::GateServiceType,
        message::{Message},

        runtime::buf_process,
        NetworkApplication, module::with_mod_ctx,
    },
    prelude::{ChannelRef, EventLifecycle, ModuleRef},
    runtime::{EventSet, EventSink, Runtime},
    time::SimTime, tracing::enter_scope,
};

///
/// The event set for a [`NetworkApplication`].
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
pub enum NetEvents {
    MessageAtGateEvent(MessageAtGateEvent),
    HandleMessageEvent(HandleMessageEvent),
    ChannelUnbusyNotif(ChannelUnbusyNotif),
    ModuleRestartEvent(ModuleRestartEvent),
    #[cfg(feature = "async")]
    AsyncWakeupEvent(AsyncWakeupEvent),
}

impl<A> EventSet<NetworkApplication<A>> for NetEvents
where
    A: EventLifecycle<NetworkApplication<A>>,
{
    fn handle(self, rt: &mut Runtime<NetworkApplication<A>>) {
        match self {
            Self::MessageAtGateEvent(event) => event.handle(rt),
            Self::HandleMessageEvent(event) => event.handle(rt),
            Self::ChannelUnbusyNotif(event) => event.handle(rt),
            Self::ModuleRestartEvent(event) => event.handle(rt),
            #[cfg(feature = "async")]
            Self::AsyncWakeupEvent(event) => event.handle(rt),
        }
    }
}

#[derive(Debug)]
pub struct MessageAtGateEvent {
    pub(crate) gate: GateRef,
    pub(crate) msg: Message,
}

impl MessageAtGateEvent {
    // This function executes an event with a sink not a runtime as an parameter.
    // That allows for the executing of events not handles by the runtime itself
    // aka. the calling with an abitrary event sink.
    pub(crate) fn handle_with_sink(self, sink: &mut impl EventSink<NetEvents>)
    {
        let mut msg = self.msg;
        msg.header.last_gate = Some(self.gate.clone());

        // Follow gates until either the gate chain is terminated,
        // or a delayed action is required.
        let mut cur = self.gate;
        while let Some(next) = cur.next_gate() {
            enter_scope(cur.owner().scope_token());

            // Since a next gate exists log the current gate as
            // transit complete. (do this before drop check to allow for better debugging at drop)
            msg.header.last_gate = Some(next.clone());

            // Drop message is owner is not active, but notfiy since this is an irregularity.
            // TODO: maybe move to log::trace if this becomes a common pattern
            if !cur.owner().is_active() {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Gate '{}' dropped message [{}] since owner module {} is inactive",
                    cur.name(),
                    msg.str(),
                    cur.owner().path()
                );

                drop(msg);
                return;
            }

            // Log the current transition to the internal log stream.
            #[cfg(feature = "tracing")]
            tracing::info!(
                "Gate '{}' forwarding message [{}] to next gate delayed: {}",
                cur.name(),
                msg.str(),
                cur.channel().is_some()
            );

            if let Some(ch) = cur.channel_mut() {
                // since a channel is nessecary for this hop, a delayed
                // action is nessecary
                assert_ne!(
                    cur.service_type(),
                    GateServiceType::Input,
                    "Channels cannot start at a input gate"
                );

                ch.send_message(msg, &next, sink);
                return;
            } 
            
            // No channel means next hop is on the same time slot,
            // so continue.
            cur = next;
        }

        // The loop has ended. This means we are at the end of a gate chain
        // cur has not been checked for anything
        enter_scope(cur.owner().scope_token());

        assert_ne!(
            cur.service_type(), 
            GateServiceType::Output,
            "Messages cannot be forwarded to modules on Output gates. (Gate '{}' owned by Module '{}')",
            cur.str(),
            cur.owner().as_str()
        );
       
        #[cfg(feature = "tracing")]
        tracing::info!(
            "Gate '{}' forwarding message [{}] to module #{}",
            cur.name(),
            msg.str(),
            cur.owner().ctx.id
        );

        let module = cur.owner();
        sink.add(
            NetEvents::HandleMessageEvent(HandleMessageEvent {
                module,
                message: msg,
            }),
            SimTime::now(),
        );


    }
}

impl MessageAtGateEvent {
    fn handle<A>(self, rt: &mut Runtime<NetworkApplication<A>>)
    where
        A: EventLifecycle<NetworkApplication<A>>,
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
    fn handle<A>(self, rt: &mut Runtime<NetworkApplication<A>>)
    where
        A: EventLifecycle<NetworkApplication<A>>,
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

        buf_process(&module, rt);
    }
}

#[derive(Debug)]
pub struct ModuleRestartEvent {
    pub(crate) module: ModuleRef
}

impl ModuleRestartEvent {
    fn handle<A>(self, rt: &mut Runtime<NetworkApplication<A>>)
    where
        A: EventLifecycle<NetworkApplication<A>>,
    {
        enter_scope(self.module.scope_token());

        #[cfg(feature = "tracing")]
        tracing::info!("ModuleRestartEvent");

        let module = &self.module;
        module.activate();
        module.module_restart();
        module.deactivate(rt);

        buf_process(&module, rt);
    }
}

#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncWakeupEvent {
    pub(crate) module: ModuleRef
}

#[cfg(feature = "async")]
impl AsyncWakeupEvent {
    fn handle<A>(self, rt: &mut Runtime<NetworkApplication<A>>)
    where
        A: EventLifecycle<NetworkApplication<A>>,
    {
        enter_scope(self.module.scope_token());

        #[cfg(feature = "tracing")]
        tracing::info!("async wakeup");

        let module = &self.module;
        module.activate();
        module.async_wakeup();
        module.deactivate(rt);

        buf_process(&module, rt);
    }
}

#[derive(Debug)]
pub struct ChannelUnbusyNotif {
    pub(crate) channel: ChannelRef,
}

impl ChannelUnbusyNotif {
    fn handle<A>(self, rt: &mut Runtime<NetworkApplication<A>>)
    where
        A: EventLifecycle<NetworkApplication<A>>,
    {
        self.channel.unbusy(rt);
    }
}

impl ModuleRef {
    pub(crate) fn num_sim_start_stages(&self) -> usize {
        self.handler.borrow().num_sim_start_stages()
    }

    pub(crate) fn reset(&self) {
        let mut brw = self.handler.borrow_mut();
        brw.reset();
    }

    // MARKER: handle_message

    #[allow(clippy::unused_self)]
    pub(crate) fn plugin_upstream(&self, msg: Option<Message>) -> Option<Message> {
        with_mod_ctx(|ctx| {
            ctx.plugins.borrow_mut().being_upstream(false);
            loop {
                let plugin = ctx.plugins.borrow_mut().next_upstream();
                let Some(mut plugin) = plugin else { break };

                plugin.event_start();
                ctx.plugins.borrow_mut().put_back_upstream(plugin);   
            }

            // Reset the upstream for message parsing
            ctx.plugins.borrow_mut().being_upstream(true);

            let mut msg = msg;
            while let Some(moved_message) = msg.take() {
                // log::trace!("capture clause");
                let plugin = ctx.plugins.borrow_mut().next_upstream();
                let Some(mut plugin) = plugin else {
                    // log::info!("noplugin");
                    msg = Some(moved_message);
                    break
                };
            
                let rem_msg = plugin.capture_incoming(moved_message);
                // log::trace!("returned some = {}", remaining_msg.is_some());
                msg = rem_msg;
                ctx.plugins.borrow_mut().put_back_upstream(plugin); 
            }
            msg
        })
}

    #[allow(clippy::unused_self)]
    pub(crate) fn plugin_downstream(&self) {
        with_mod_ctx(|ctx|{
            ctx.plugins.borrow_mut().begin_main_downstream();
            loop {
                let plugin = ctx.plugins.borrow_mut().next_downstream();
                let Some(mut plugin) = plugin else { break };

                plugin.event_end();
                ctx.plugins.borrow_mut().put_back_downstream(plugin, true);
                
            }
        })
    }

    #[cfg(feature = "async")]
    pub(crate) fn async_wakeup(&self) {
        if self.ctx.active.load(SeqCst) {
            let _ = self.plugin_upstream(None);
            if self.handler.borrow().__indicate_async() {
                Self::run_without_event()
            }
            self.plugin_downstream();
        }else {
            #[cfg(feature = "tracing")]
            tracing::debug!("Ignoring message since module is inactive");
        }
    } 

    #[cfg(feature = "async")]
    fn run_without_event() {  
        use tokio::task::yield_now;
        use crate::net::module::async_get_rt;
        let Some(rt) = async_get_rt() else {
            return
        };
        rt.1.block_on(&rt.0, yield_now());
    }


    pub(crate) fn module_restart(&self) {
         // TODO: verify
         #[cfg(feature = "tracing")]
         tracing::debug!("Restarting module");
         // restart the module itself.
         // self.reset();
         self.ctx.active.store(true, SeqCst);

         // Do sim start procedure
         let stages = self.num_sim_start_stages();
         for stage in 0..stages {
             self.at_sim_start(stage);
         }

         #[cfg(feature = "async")]
         self.finish_sim_start();
    }

    pub(crate) fn handle_message(&self, msg: Message) {
        if self.ctx.active.load(SeqCst) {
            // (0) Run upstream plugins.
            let msg = self.plugin_upstream(Some(msg));

            // (1) Call handle message, if the message was not consumed
            // - If async and the message was consumed, send a NOTIFY packet to
            //   still call poll until idle, and internal RT management.
            if let Some(msg) = msg {
                self.handler.borrow_mut().handle_message(msg);
            } else {
                #[cfg(feature = "async")]
                if self.handler.borrow().__indicate_async() {
                    Self::run_without_event()
                }
            }

            // (2) Plugin downstram operations
            self.plugin_downstream();
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!("Ignoring message since module is inactive");
        }
    }

    pub(crate) fn at_sim_start(&self, stage: usize) {
        self.plugin_upstream(None);
        self.handler.borrow_mut().at_sim_start(stage);
        self.plugin_downstream();
    }

    #[cfg(feature = "async")]
    pub(crate) fn finish_sim_start(&self) {
        if self.handler.borrow().__indicate_async() {
            self.plugin_upstream(None);
            self.handler.borrow_mut().finish_sim_start();
            self.plugin_downstream();
        }
    }

    pub(crate) fn at_sim_end(&self) {
        self.plugin_upstream(None);
        self.handler.borrow_mut().at_sim_end();
        self.plugin_downstream();
    }

    #[cfg(feature = "async")]
    pub(crate) fn finish_sim_end(&self) {
        if self.handler.borrow().__indicate_async() {
            self.plugin_upstream(None);
            self.handler.borrow_mut().finish_sim_end();
            self.plugin_downstream();
        }
    }
}
