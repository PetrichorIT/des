use log::{info, warn};

use crate::{core::*, create_event_set, net::*, util::MrcS};

create_event_set!(
    ///
    /// The event set for a [NetworkRuntime].
    ///
    /// * This type is only available of DES is build with the `"net"` feature.
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub enum NetEvents {
        type App = NetworkRuntime<A>;

        MessageAtGateEvent(MessageAtGateEvent),
        HandleMessageEvent(HandleMessageEvent),
        CoroutineMessageEvent(CoroutineMessageEvent),
        ChannelUnbusyNotif(ChannelUnbusyNotif),
        SimStartNotif(SimStartNotif),
    };
);

pub struct MessageAtGateEvent {
    pub gate: GateRef,
    pub message: Message,
}

impl<A> Event<NetworkRuntime<A>> for MessageAtGateEvent {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        let mut message = self.message;
        message.meta.last_gate = Some(MrcS::clone(&self.gate));

        //
        // Iterate through gates until:
        // a) a final gate with no next_gate was found, indicating a handle_module_call
        // b) a delay gate was found, apply the delay and recall in a new event.
        //
        let mut current_gate = &self.gate;
        while let Some(next_gate) = current_gate.next_gate() {
            // A next gate exists.
            // redirect to next channel
            message.meta.last_gate = Some(MrcS::clone(next_gate));

            info!(
                target: &format!("Gate ({})", current_gate.name()),
                "Forwarding message [{}] to next gate delayed: {}",
                message.str(),
                current_gate.channel().is_some()
            );

            match current_gate.channel_mut() {
                Some(mut channel) => {
                    // Channel delayed connection
                    assert!(
                        current_gate.service_type() != GateServiceType::Input,
                        "Channels cannot start at a input node"
                    );

                    let mut ptr = get_rtc_ptr();
                    let rng_ref = &mut ptr.as_mut().unwrap().rng;

                    if channel.is_busy() {
                        warn!(
                            target: &format!("Gate #{}", current_gate.name()),
                            "Dropping message {} pushed onto busy channel #{:?}",
                            message.str(),
                            channel
                        );
                        drop(message);
                        return;
                    }

                    let dur = channel.calculate_duration(&message, rng_ref);
                    let busy = channel.calculate_busy(&message);

                    let transmissin_finish = SimTime::now() + busy;

                    channel.set_busy_until(transmissin_finish);

                    rt.add_event(
                        NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif { channel }),
                        transmissin_finish,
                    );

                    let next_event_time = SimTime::now() + dur;

                    rt.add_event(
                        NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                            gate: MrcS::clone(next_gate),
                            message,
                        }),
                        next_event_time,
                    );

                    // must break iteration,
                    // but not perform on-module handling
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

        assert!(
            current_gate.service_type() != GateServiceType::Output,
            "Messages cannot be forwarded to modules on Output gates. (Gate '{}' owned by Module '{}')",
            current_gate.str(),
            current_gate.owner().str()
        );

        info!(
            target: &format!("Gate ({})", current_gate.name()),
            "Forwarding message [{}] to module #{}",
            message.str(),
            current_gate.owner().id()
        );

        let module = MrcS::clone(current_gate.owner());
        rt.add_event(
            NetEvents::HandleMessageEvent(HandleMessageEvent { module, message }),
            SimTime::now(),
        );
    }
}

pub struct HandleMessageEvent {
    pub module: ModuleRefMut,
    pub message: Message,
}

impl<A> Event<NetworkRuntime<A>> for HandleMessageEvent {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        let mut message = self.message;
        message.meta.receiver_module_id = self.module.id();

        info!(
            target: &format!("Module {}", self.module.str()),
            "Handling message {:?}",
            message.str()
        );

        let mut module = MrcS::clone(&self.module);

        module.prepare_buffers();
        module.handle_message(message);
        module.handle_buffers(rt);
    }
}

pub struct CoroutineMessageEvent {
    module: ModuleRefMut,
}

impl<A> Event<NetworkRuntime<A>> for CoroutineMessageEvent {
    fn handle(mut self, rt: &mut Runtime<NetworkRuntime<A>>) {
        let dur = self.module.module_core().activity_period;
        // This message can only occure *after* the activity is
        // fully initalized.
        // It can be the case that this is wrong .. if handle_message at invalidated activity
        // but a event still remains in queue.
        if dur != SimTime::ZERO && self.module.module_core().activity_active {
            self.module.prepare_buffers();
            self.module.activity();

            let still_active = self.module.module_core().activity_active;

            // This call will only use up out buffers and loopback buffers
            // not schedule a new Coroutine since either:
            // - state remains stable since allready init
            // - activity deactivated.
            MrcS::clone(&self.module).handle_buffers(rt);

            if still_active {
                rt.add_event_in(
                    NetEvents::CoroutineMessageEvent(CoroutineMessageEvent {
                        module: self.module,
                    }),
                    dur,
                )
            }
        }
    }
}

pub struct ChannelUnbusyNotif {
    channel: ChannelRefMut,
}

impl<A> Event<NetworkRuntime<A>> for ChannelUnbusyNotif {
    fn handle(mut self, _rt: &mut Runtime<NetworkRuntime<A>>) {
        self.channel.unbusy()
    }
}

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
                let mut module = MrcS::clone(&rt.app.modules()[i]);
                if stage < module.num_sim_start_stages() {
                    info!(
                        target: &format!("Module: {}", module.str()),
                        "Calling at_sim_start({}).", stage
                    );
                    module.prepare_buffers();
                    module.at_sim_start(stage);
                    module.handle_buffers(rt);
                }
            }
        }
    }
}

//
// # Helper functions
//

impl ModuleRefMut {
    fn prepare_buffers(&mut self) {
        self.processing_time_delay = SimTime::ZERO;
    }

    fn handle_buffers<A>(&mut self, rt: &mut Runtime<NetworkRuntime<A>>) {
        // Check whether a new activity cycle must be initated
        let enqueue_actitivy_msg = self.module_core().activity_period != SimTime::ZERO
            && !self.module_core().activity_active;

        self.module_core_mut().activity_active = true;

        let self_id = self.id();

        // Send gate events from the 'send' method calls
        for (mut message, gate, offset) in self.module_core_mut().out_buffer.drain(..) {
            assert!(
                gate.service_type() != GateServiceType::Input,
                "To send messages onto a gate it must have service type of 'Output' or 'Undefined'"
            );
            message.meta.sender_module_id = self_id;
            rt.add_event(
                NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                    // TODO
                    gate,
                    message,
                }),
                SimTime::now() + offset,
            )
        }

        let mref = MrcS::clone(self);

        // Send loopback events from 'scheduleAt'
        for (message, time) in self.module_core_mut().loopback_buffer.drain(..) {
            rt.add_event(
                NetEvents::HandleMessageEvent(HandleMessageEvent {
                    module: MrcS::clone(&mref),
                    message,
                }),
                time,
            )
        }

        // initalily
        // call activity
        if enqueue_actitivy_msg {
            rt.add_event_in(
                NetEvents::CoroutineMessageEvent(CoroutineMessageEvent { module: mref }),
                self.module_core().activity_period,
            )
        }
    }
}
