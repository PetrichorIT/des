use std::panic;

use log::info;

use crate::{
    create_event_set,
    net::{
        gate::GateRef,
        gate::GateServiceType,
        message::{Message, TYP_RESTART},
        module::with_mod_ctx,
        plugin::UnwindSafeBox,
        runtime::buf_process,
        NetworkRuntime,
    },
    prelude::{ChannelRef, ModuleRef},
    runtime::{Event, EventSet, Runtime},
    time::SimTime,
};

create_event_set!(
    ///
    /// The event set for a [`NetworkRuntime`].
    ///
    /// * This type is only available of DES is build with the `"net"` feature.
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    #[derive(Debug)]
    pub enum NetEvents {
        type App = NetworkRuntime<A>;

        MessageAtGateEvent(MessageAtGateEvent),
        HandleMessageEvent(HandleMessageEvent),
        ChannelUnbusyNotif(ChannelUnbusyNotif),
        SimStartNotif(SimStartNotif),
    };
);

#[derive(Debug)]
pub struct MessageAtGateEvent {
    pub(crate) gate: GateRef,
    pub(crate) message: Box<Message>,
}

impl<A> Event<NetworkRuntime<A>> for MessageAtGateEvent {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        let mut message = self.message;
        message.header.last_gate = Some(GateRef::clone(&self.gate));

        //
        // Iterate through gates until:
        // a) a final gate with no next_gate was found, indicating a handle_module_call
        // b) a delay gate was found, apply the delay and recall in a new event.
        //
        let mut current_gate = self.gate;
        while let Some(next_gate) = current_gate.next_gate() {
            log_scope!(current_gate.owner().ctx.path.path());

            // A next gate exists.
            // redirect to next channel
            message.header.last_gate = Some(GateRef::clone(&next_gate));

            info!(
                "Gate '{}' forwarding message [{}] to next gate delayed: {}",
                current_gate.name(),
                message.str(),
                current_gate.channel().is_some()
            );

            match current_gate.channel_mut() {
                Some(channel) => {
                    // Channel delayed connection
                    assert!(
                        current_gate.service_type() != GateServiceType::Input,
                        "Channels cannot start at a input node"
                    );

                    channel.send_message(message, &next_gate, rt);
                    return;
                }
                None => {
                    // no delay nessecary
                    // goto next iteration
                    current_gate = next_gate;
                }
            }
        }

        // No next gate exists.
        debug_assert!(current_gate.next_gate().is_none());
        log_scope!(current_gate.owner().ctx.path.path());

        assert!(
            current_gate.service_type() != GateServiceType::Output,
            "Messages cannot be forwarded to modules on Output gates. (Gate '{}' owned by Module '{}')",
            current_gate.str(),
            current_gate.owner().str()
        );

        info!(
            "Gate '{}' forwarding message [{}] to module #{}",
            current_gate.name(),
            message.str(),
            current_gate.owner().ctx.id
        );

        let module = current_gate.owner();
        rt.add_event(
            NetEvents::HandleMessageEvent(HandleMessageEvent { module, message }),
            SimTime::now(),
        );

        log_scope!();
    }
}

#[derive(Debug)]
pub struct HandleMessageEvent {
    pub(crate) module: ModuleRef,
    pub(crate) message: Box<Message>,
}

impl<A> Event<NetworkRuntime<A>> for HandleMessageEvent {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        log_scope!(self.module.str());
        let mut message = *self.message;
        message.header.receiver_module_id = self.module.ctx.id;

        info!("Handling message {:?}", message.str());

        let module = self.module;

        module.activate();
        module.handle_message2(message);
        module.deactivate();

        buf_process(&module, rt);

        log_scope!();
    }
}

#[derive(Debug)]
pub struct ChannelUnbusyNotif {
    pub(crate) channel: ChannelRef,
}

impl<A> Event<NetworkRuntime<A>> for ChannelUnbusyNotif {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        self.channel.unbusy(rt);
    }
}

#[derive(Debug)]
pub struct SimStartNotif();

impl<A> Event<NetworkRuntime<A>> for SimStartNotif {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        // This is a explicit for loop to prevent borrow rt only in the inner block
        // allowing preemtive dropping of 'module' so that rt can be used in
        // 'module_handle_jobs'.
        let max_stage = rt
            .app
            .modules()
            .iter()
            .fold(1, |acc, module| acc.max(module.num_sim_start_stages()));

        for stage in 0..max_stage {
            // Direct indexing since rt must be borrowed mutably in handle_buffers.
            for i in 0..rt.app.modules().len() {
                let module = rt.app.modules()[i].clone();
                log_scope!(module.ctx.path.path());

                if stage < module.num_sim_start_stages() {
                    info!("Calling at_sim_start({}).", stage);

                    module.activate();
                    module.at_sim_start2(stage);
                    module.deactivate();

                    super::buf_process(&module, rt);
                }
            }
        }

        #[cfg(feature = "async")]
        {
            // Ensure all sim_start stages have finished

            for i in 0..rt.app.modules().len() {
                let module = rt.app.modules()[i].clone();
                log_scope!(module.ctx.path.path());

                module.activate();
                module.finish_sim_start2();
                module.deactivate();

                super::buf_process(&module, rt);
            }
        }
        log_scope!();
    }
}

impl ModuleRef {
    pub(crate) fn num_sim_start_stages(&self) -> usize {
        self.handler.borrow().num_sim_start_stages()
    }

    pub(crate) fn reset(&self) {
        self.handler.borrow_mut().reset();
    }

    // MARKER: handle_message

    #[allow(clippy::unused_self)]
    pub(crate) fn plugin_upstream(&self, msg: Option<Message>) -> Option<Message> {
        with_mod_ctx(|ctx| ctx.plugins.write().being_upstream(false));
        loop {
            let plugin = with_mod_ctx(|ctx| ctx.plugins.write().next_upstream());
            let Some(plugin) = plugin else { break };
            let plugin = UnwindSafeBox(plugin);

            let result = panic::catch_unwind(move || {
                let mut plugin = plugin;
                plugin.0.event_start();
                plugin
            });

            match result {
                Ok(plugin) => {
                    with_mod_ctx(|ctx| ctx.plugins.write().put_back_upstream(plugin.0));
                }
                Err(p) => {
                    with_mod_ctx(|ctx| ctx.plugins.write().paniced_upstream(p));
                }
            }
        }

        // Reset the upstream for message parsing
        with_mod_ctx(|ctx| ctx.plugins.write().being_upstream(true));

        let mut msg = msg;
        while let Some(moved_message) = msg.take() {
            // log::trace!("capture clause");
            let plugin = with_mod_ctx(|ctx| ctx.plugins.write().next_upstream());
            let Some(plugin) = plugin else {
                // log::info!("noplugin");
                msg = Some(moved_message);
                break
            };
            let plugin = UnwindSafeBox(plugin);

            let result = panic::catch_unwind(move || {
                let mut plugin = plugin;
                let ret = plugin.0.capture_incoming(moved_message);
                (ret, plugin)
            });

            match result {
                Ok((remaining_msg, plugin)) => {
                    // log::trace!("returned some = {}", remaining_msg.is_some());
                    msg = remaining_msg;
                    with_mod_ctx(|ctx| ctx.plugins.write().put_back_upstream(plugin.0));
                }
                Err(p) => {
                    with_mod_ctx(|ctx| ctx.plugins.write().paniced_upstream(p));
                }
            }
        }
        msg
    }

    #[allow(clippy::unused_self)]
    pub(crate) fn plugin_downstream(&self) {
        with_mod_ctx(|ctx| ctx.plugins.write().begin_main_downstream());
        loop {
            let plugin = with_mod_ctx(|ctx| ctx.plugins.write().next_downstream());
            let Some(plugin) = plugin else { break };
            let plugin = UnwindSafeBox(plugin);

            let result = panic::catch_unwind(move || {
                let mut plugin = plugin;
                plugin.0.event_end();
                plugin
            });

            match result {
                Ok(plugin) => {
                    with_mod_ctx(|ctx| ctx.plugins.write().put_back_downstream(plugin.0, true));
                }
                Err(p) => {
                    with_mod_ctx(|ctx| ctx.plugins.write().paniced_downstream(p));
                    continue;
                }
            }
        }
    }

    pub(crate) fn handle_message2(&self, msg: Message) {
        use std::sync::atomic::Ordering::SeqCst;
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
                if self.handler.borrow().__indicate_asnyc() {
                    self.handler.borrow_mut().handle_message(Message::notify());
                }
            }

            // (2) Plugin downstram operations
            self.plugin_downstream();
        } else if msg.header().typ == TYP_RESTART {
            // TODO: verify
            log::debug!("Restarting module");
            // restart the module itself.
            self.reset();
            self.ctx.active.store(true, SeqCst);

            // Do sim start procedure
            let stages = self.num_sim_start_stages();
            for stage in 0..stages {
                self.at_sim_start2(stage);
            }

            #[cfg(feature = "async")]
            self.finish_sim_start2();
        } else {
            log::debug!("Ignoring message since module is inactive");
        }
    }

    pub(crate) fn at_sim_start2(&self, stage: usize) {
        self.plugin_upstream(None);
        self.handler.borrow_mut().at_sim_start(stage);
        self.plugin_downstream();
    }

    #[cfg(feature = "async")]
    pub(crate) fn finish_sim_start2(&self) {
        if self.handler.borrow().__indicate_asnyc() {
            self.plugin_upstream(None);
            self.handler.borrow_mut().finish_sim_start();
            self.plugin_downstream();
        }
    }

    pub(crate) fn at_sim_end2(&self) {
        self.plugin_upstream(None);
        self.handler.borrow_mut().at_sim_end();
        self.plugin_downstream();
    }

    #[cfg(feature = "async")]
    pub(crate) fn finish_sim_end2(&self) {
        if self.handler.borrow().__indicate_asnyc() {
            self.plugin_upstream(None);
            self.handler.borrow_mut().finish_sim_end();
            self.plugin_downstream();
        }
    }

    // OLD IMPL

    // Handles a message
    // pub(crate) fn handle_message(&self, msg: Message) {
    //     use std::sync::atomic::Ordering::SeqCst;

    //     if self.ctx.active.load(SeqCst) {
    //         // (0) Run all plugins upward
    //         // Call in order from lowest to highest priority.
    //         let msg = with_mod_ctx(|ctx| {
    //             let mut plugins = ctx.plugins.write();
    //             let mut msg = Some(msg);
    //             for plugin in plugins.iter_mut() {
    //                 if !plugin.just_created {
    //                     msg = plugin.try_capture(msg);
    //                 }
    //             }
    //             msg
    //         });

    //         // Call handle message, if the message was not consumed
    //         // - If async and the message was consumed, send a NOTIFY packet to
    //         //   still call poll until idle, and internal RT management.
    //         if let Some(msg) = msg {
    //             self.handler.borrow_mut().handle_message(msg);
    //         } else {
    //             #[cfg(feature = "async")]
    //             if self.handler.borrow().__indicate_asnyc() {
    //                 self.handler.borrow_mut().handle_message(Message::notify());
    //             }
    //         }

    //         // (2) Plugin defer calls
    //         // Call in reverse order to preserve user-space distance
    //         with_mod_ctx(|ctx| {
    //             for plugin in ctx.plugins.write().iter_mut().rev() {
    //                 if !plugin.just_created {
    //                     plugin.try_defer();
    //                 }
    //                 plugin.just_created = false;
    //             }
    //         });
    //     } else if msg.header().typ == TYP_RESTART {
    //         log::debug!("Restarting module");
    //         // restart the module itself.
    //         self.reset();
    //         self.ctx.active.store(true, SeqCst);

    //         // Do sim start procedure
    //         let stages = self.num_sim_start_stages();
    //         for stage in 0..stages {
    //             self.at_sim_start(stage);
    //         }

    //         #[cfg(feature = "async")]
    //         self.finish_sim_start();
    //     } else {
    //         log::debug!("Ignoring message since module is inactive");
    //     }
    // }

    // pub(crate) fn at_sim_start(&self, stage: usize) {
    //     // (0) Run all plugins upward
    //     // Call in order from lowest to highest priority.
    //     with_mod_ctx(|ctx| {
    //         for plugin in ctx.plugins.write().iter_mut() {
    //             if !plugin.just_created {
    //                 plugin.try_capture_sim_start();
    //             }
    //         }
    //     });

    //     // (1) Calle the underlining impl
    //     self.handler.borrow_mut().at_sim_start(stage);

    //     // (2) Plugin defer calls
    //     // Call in reverse order to preserve user-space distance
    //     with_mod_ctx(|ctx| {
    //         for plugin in ctx.plugins.write().iter_mut().rev() {
    //             if !plugin.just_created {
    //                 plugin.try_defer_sim_start();
    //             }
    //             plugin.just_created = false;
    //         }
    //     });
    // }

    // #[cfg(feature = "async")]
    // pub(crate) fn finish_sim_start(&self) {
    //     // (0) Run all plugins upward
    //     // Call in order from lowest to highest priority.
    //     with_mod_ctx(|ctx| {
    //         for plugin in ctx.plugins.write().iter_mut() {
    //             if !plugin.just_created {
    //                 plugin.try_capture_sim_start();
    //             }
    //         }
    //     });

    //     // (1) Calle the underlining impl
    //     self.handler.borrow_mut().finish_sim_start();

    //     // (2) Plugin defer calls
    //     // Call in reverse order to preserve user-space distance
    //     with_mod_ctx(|ctx| {
    //         for plugin in ctx.plugins.write().iter_mut().rev() {
    //             if !plugin.just_created {
    //                 plugin.try_defer_sim_start();
    //             }
    //             plugin.just_created = false;
    //         }
    //     });
    // }

    // pub(crate) fn at_sim_end(&self) {
    //     // (0) Run all plugins upward
    //     // Call in order from lowest to highest priority.
    //     with_mod_ctx(|ctx| {
    //         for plugin in ctx.plugins.write().iter_mut() {
    //             if !plugin.just_created {
    //                 plugin.try_capture_sim_end();
    //             }
    //         }
    //     });

    //     // (1) Call inner
    //     self.handler.borrow_mut().at_sim_end();

    //     // (2) Plugin defer calls
    //     // Call in reverse order to preserve user-space distance
    //     with_mod_ctx(|ctx| {
    //         for plugin in ctx.plugins.write().iter_mut().rev() {
    //             if !plugin.just_created {
    //                 plugin.try_defer_sim_end();
    //             }
    //             plugin.just_created = false;
    //         }
    //     });
    // }

    // #[cfg(feature = "async")]
    // pub(crate) fn finish_sim_end(&self) {
    //     // (0) Run all plugins upward
    //     // Call in order from lowest to highest priority.
    //     with_mod_ctx(|ctx| {
    //         for plugin in ctx.plugins.write().iter_mut() {
    //             if !plugin.just_created {
    //                 plugin.try_capture_sim_end();
    //             }
    //         }
    //     });

    //     // (1) Call inner
    //     self.handler.borrow_mut().finish_sim_end();

    //     // (2) Plugin defer calls
    //     // Call in reverse order to preserve user-space distance
    //     with_mod_ctx(|ctx| {
    //         for plugin in ctx.plugins.write().iter_mut().rev() {
    //             if !plugin.just_created {
    //                 plugin.try_defer_sim_end();
    //             }
    //             plugin.just_created = false;
    //         }
    //     });
    // }
}
