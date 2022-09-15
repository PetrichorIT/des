use log::info;

use crate::{
    create_event_set,
    net::{runtime::buf_process, GateRef, GateServiceType, Message, NetworkRuntime},
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
    pub(crate) message: Message,
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
    pub(crate) message: Message,
}

impl<A> Event<NetworkRuntime<A>> for HandleMessageEvent {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        log_scope!(self.module.str());
        let mut message = self.message;
        message.header.receiver_module_id = self.module.ctx.id;

        info!("Handling message {:?}", message.str());

        let module = self.module;

        #[cfg(feature = "metrics-module-time")]
        use std::time::Instant;
        #[cfg(feature = "metrics-module-time")]
        let t0 = Instant::now();

        module.activate();
        module.handler().handle_message(message);
        module.deactivate();

        buf_process(&module, rt);

        #[cfg(feature = "metrics-module-time")]
        {
            let elapsed = Instant::now().duration_since(t0);
            module.module_core_mut().elapsed += elapsed;
            rt.app.globals.time_elapsed += elapsed;
        }

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
        let max_stage = rt.app.modules().iter().fold(1, |acc, module| {
            acc.max(module.handler().num_sim_start_stages())
        });

        for stage in 0..max_stage {
            // Direct indexing since rt must be borrowed mutably in handle_buffers.
            for i in 0..rt.app.modules().len() {
                let module = rt.app.modules()[i].clone();
                log_scope!(module.ctx.path.path());

                if stage < module.handler().num_sim_start_stages() {
                    info!("Calling at_sim_start({}).", stage);

                    #[cfg(feature = "metrics-module-time")]
                    use std::time::Instant;
                    #[cfg(feature = "metrics-module-time")]
                    let t0 = Instant::now();

                    module.activate();
                    module.handler().at_sim_start(stage);
                    module.deactivate();

                    super::buf_process(&module, rt);

                    #[cfg(feature = "metrics-module-time")]
                    {
                        let elapsed = Instant::now().duration_since(t0);
                        module.module_core_mut().elapsed += elapsed;
                        rt.app.globals.time_elapsed += elapsed;
                    }
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
                module.handler().finish_sim_start();
                module.deactivate();

                // TODO: Is this really nessecary?
                super::buf_process(&module, rt);
            }
        }

        log_scope!();
    }
}
