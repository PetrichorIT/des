use log::{info, warn};

use crate::{create_event_set, net::*, runtime::*, time::*, util::*};

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
        message.meta.last_gate = Some(Ptr::clone(&self.gate));

        //
        // Iterate through gates until:
        // a) a final gate with no next_gate was found, indicating a handle_module_call
        // b) a delay gate was found, apply the delay and recall in a new event.
        //
        let mut current_gate = &self.gate;
        while let Some(next_gate) = current_gate.next_gate() {
            // A next gate exists.
            // redirect to next channel
            message.meta.last_gate = Some(Ptr::clone(next_gate));

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

                    let rng_ref = rng();

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
                            gate: Ptr::clone(next_gate),
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

        let module = PtrWeakMut::clone(current_gate.owner());
        rt.add_event(
            NetEvents::HandleMessageEvent(HandleMessageEvent { module, message }),
            SimTime::now(),
        );
    }
}

pub struct HandleMessageEvent {
    pub module: PtrWeakMut<dyn Module>,
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

        let mut module = PtrWeakMut::clone(&self.module);

        module.prepare_buffers();
        module.handle_message(message);
        module.handle_buffers(rt);
    }
}

pub struct CoroutineMessageEvent {
    module: PtrWeakMut<dyn Module>,
}

impl<A> Event<NetworkRuntime<A>> for CoroutineMessageEvent {
    fn handle(mut self, rt: &mut Runtime<NetworkRuntime<A>>) {
        let dur = self.module.module_core().activity_period;
        // This message can only occure *after* the activity is
        // fully initalized.
        // It can be the case that this is wrong .. if handle_message at invalidated activity
        // but a event still remains in queue.
        if dur != Duration::ZERO && self.module.module_core().activity_active {
            self.module.prepare_buffers();
            self.module.activity();

            let still_active = self.module.module_core().activity_active;

            // This call will only use up out buffers and loopback buffers
            // not schedule a new Coroutine since either:
            // - state remains stable since allready init
            // - activity deactivated.
            PtrWeakMut::clone(&self.module).handle_buffers(rt);

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
                let mut module = PtrWeakMut::from_strong(&rt.app.modules()[i]);
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

        #[cfg(feature = "async")]
        {
            // Ensure all sim_start stages have finished

            for i in 0..rt.app.modules().len() {
                let mut module = PtrWeakMut::from_strong(&rt.app.modules()[i]);
                module.finish_sim_start()
            }
        }
    }
}

//
// # Helper functions
//

impl PtrWeakMut<dyn Module> {
    fn prepare_buffers(&mut self) {
        self.buffers.processing_time_delay = Duration::ZERO;
    }

    fn handle_buffers<A>(&mut self, rt: &mut Runtime<NetworkRuntime<A>>) {
        // Check whether a new activity cycle must be initated
        let enqueue_actitivy_msg = self.module_core().activity_period != Duration::ZERO
            && !self.module_core().activity_active;

        self.module_core_mut().activity_active = true;

        let self_id = self.id();
        let mref = PtrWeakMut::clone(self);

        // Drain the buffers from the async handle
        #[cfg(feature = "async")]
        {
            use crate::net::module::*;

            while let Ok(ev) = self.module_core_mut().async_ext.buffers.try_recv() {
                match ev {
                    BufferEvent::Send {
                        mut msg,
                        time_offset,
                        out,
                    } => {
                        let gate = out
                            .as_gate(&self.module_core())
                            .expect("Async buffers failed to resolve out parameter");

                        assert!(
                                gate.service_type() != GateServiceType::Input,
                                "To send messages onto a gate it must have service type of 'Output' or 'Undefined'"
                            );
                        msg.meta.sender_module_id = self_id;
                        rt.add_event(
                            NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                                gate,
                                message: msg,
                            }),
                            SimTime::now() + time_offset,
                        )
                    }

                    BufferEvent::ScheduleIn { msg, time_offset } => rt.add_event(
                        NetEvents::HandleMessageEvent(HandleMessageEvent {
                            module: PtrWeakMut::clone(&mref),
                            message: msg,
                        }),
                        SimTime::now() + time_offset,
                    ),

                    BufferEvent::ScheduleAt { msg, time } => rt.add_event(
                        NetEvents::HandleMessageEvent(HandleMessageEvent {
                            module: PtrWeakMut::clone(&mref),
                            message: msg,
                        }),
                        time,
                    ),
                }
            }
        }

        // get drain
        let mut_ref = &mut self.module_core_mut().buffers.out_buffer;

        // Send gate events from the 'send' method calls
        while let Some((mut message, gate, offset)) = mut_ref.pop() {
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
                offset,
            )
        }
        // drop(mut_ref);

        // get drain
        let mut_ref = &mut self.module_core_mut().buffers.loopback_buffer;

        // Send loopback events from 'scheduleAt'
        while let Some((message, time)) = mut_ref.pop() {
            rt.add_event(
                NetEvents::HandleMessageEvent(HandleMessageEvent {
                    module: PtrWeakMut::clone(&mref),
                    message,
                }),
                time,
            )
        }

        // drop(mut_ref);

        // initalily
        // call activity
        if enqueue_actitivy_msg {
            rt.add_event_in(
                NetEvents::CoroutineMessageEvent(CoroutineMessageEvent { module: mref }),
                self.module_core().activity_period,
            )
        }

        if !rt.app.globals.parameters.updates.borrow().is_empty() {
            for update in rt.app.globals.parameters.updates.borrow_mut().drain(..) {
                rt.app
                    .module(|m| m.name() == update)
                    .unwrap()
                    .handle_par_change()
            }
        }
    }
}
