use log::{error, info, warn};
use std::mem::ManuallyDrop;
use std::ops::Deref;

use crate::core::*;
use crate::create_event_set;
use crate::net::*;
use crate::util::*;

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
    pub gate_id: GateId,
    pub message: ManuallyDrop<Message>,
    pub handled: bool,
}

impl<A> Event<NetworkRuntime<A>> for MessageAtGateEvent {
    fn handle(mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        let gate = rt.app.gate(self.gate_id);
        if let Some(gate) = gate {
            let ptr: *const Message = self.message.deref();
            let mut message = unsafe { std::ptr::read(ptr) };
            message.meta.last_gate = self.gate_id;

            self.handled = true;

            if gate.next_gate() == GateId::NULL {
                info!(
                    target: &format!("Gate #{} ({})", self.gate_id, gate.name()),
                    "Forwarding message [{}] to module #{}",
                    message.str(),
                    gate.module()
                );

                let module = gate.module();
                rt.add_event(
                    NetEvents::HandleMessageEvent(HandleMessageEvent {
                        module_id: module,
                        message: ManuallyDrop::new(message),
                        handled: false,
                    }),
                    SimTime::now(),
                );
            } else {
                // redirect to next channel
                let next_gate = gate.next_gate();

                info!(
                    target: &format!("Gate #{} ({})", self.gate_id, gate.name()),
                    "Forwarding message [{}] to next gate #{} delyed: {}",
                    message.str(),
                    next_gate,
                    gate.channel_id() != ChannelId::NULL
                );

                let next_event_time = if gate.channel_id() == ChannelId::NULL {
                    // Direct connection
                    SimTime::now()
                } else {
                    // Channel delayed connection
                    let channel_id = gate.channel_id();

                    // SAFTY:
                    // The rng and random number generator dont interfere so this operation can
                    // be considered safe. Make sure this ref is only used in conjunction with the channel.
                    let rng_ref = unsafe { &mut (*rt.rng()) };
                    let channel = rt.app.channel_mut(channel_id).unwrap();

                    if channel.is_busy() {
                        warn!(
                            target: &format!("Gate #{}", self.gate_id),
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
                        NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif { channel_id }),
                        transmissin_finish,
                    );

                    SimTime::now() + dur
                };

                rt.add_event(
                    NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                        gate_id: next_gate,
                        message: ManuallyDrop::new(message),
                        handled: false,
                    }),
                    next_event_time,
                )
            }
        } else {
            error!(
                target: &format!("Undefined Gate #{}", self.gate_id),
                "Message {:?} dropped after gate was not found",
                self.message.str()
            )
        }
    }
}

impl Drop for MessageAtGateEvent {
    fn drop(&mut self) {
        if !self.handled && std::mem::needs_drop::<Message>() {
            // SAFTY:
            // If the message was no forwarded to another party
            // drop it manully
            unsafe { ManuallyDrop::drop(&mut self.message) }
        }
    }
}

pub struct HandleMessageEvent {
    pub module_id: ModuleId,
    pub message: ManuallyDrop<Message>,
    pub handled: bool,
}

impl<A> Event<NetworkRuntime<A>> for HandleMessageEvent {
    fn handle(mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        if let Some(module) = rt.app.module_mut_by_id(self.module_id) {
            let ptr: *const Message = self.message.deref();
            let mut message = unsafe { std::ptr::read(ptr) };
            message.meta.receiver_module_id = module.id();

            info!(
                target: &format!("Module {}", module.str()),
                "Handling message {:?}",
                message.str()
            );

            module.handle_message(message);

            let job = module_drain_buffers(module.module_core_mut());
            module_handle_jobs(rt, job);

            self.handled = true;
        } else {
            error!(
                target: &format!("Unknown module #{}", self.module_id),
                "Dropped message {} since module was not found",
                self.message.str()
            );
        }
    }
}

impl Drop for HandleMessageEvent {
    fn drop(&mut self) {
        if !self.handled {
            unsafe { ManuallyDrop::drop(&mut self.message) }
        }
    }
}

pub struct CoroutineMessageEvent {
    module_id: ModuleId,
}

impl<A> Event<NetworkRuntime<A>> for CoroutineMessageEvent {
    fn handle(self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        if let Some(module) = rt.app.module_mut_by_id(self.module_id) {
            let dur = module.module_core().activity_period;
            // This message can only occure *after* the activity is
            // fully initalized.
            // It can be the case that this is wrong .. if handle_message at invalidated activity
            // but a event still remains in queue.
            if dur != SimTime::ZERO && module.module_core().activity_active {
                module.activity();

                let still_active = module.module_core().activity_active;

                // This call will only use up out buffers and loopback buffers
                // not schedule a new Coroutine since either:
                // - state remains stable since allready init
                // - activity deactivated.
                let jobs = module_drain_buffers(module.module_core_mut());
                module_handle_jobs(rt, jobs);

                if still_active {
                    rt.add_event_in(
                        NetEvents::CoroutineMessageEvent(CoroutineMessageEvent {
                            module_id: self.module_id,
                        }),
                        dur,
                    )
                }
            }
        } else {
            error!(
                target: &format!("Module #{}", self.module_id),
                " Coroutine call",
            );
        }
    }
}

pub struct ChannelUnbusyNotif {
    channel_id: ChannelId,
}

impl<A> Event<NetworkRuntime<A>> for ChannelUnbusyNotif {
    fn handle(self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        if let Some(channel) = rt.app.channel_mut(self.channel_id) {
            channel.unbusy();
        }
    }
}

pub struct SimStartNotif();

impl<A> Event<NetworkRuntime<A>> for SimStartNotif {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        // This is a explicit for loop to prevent borrow rt only in the inner block
        // allowing preemtive dropping of 'module' so that rt can be used in
        // 'module_handle_jobs'.
        for i in 0..rt.app.modules().len() {
            let module = &mut rt.app.modules_mut()[i];
            info!(
                target: &format!("Module {}", module.str()),
                "Calling at_sim_start."
            );
            module.at_sim_start();

            let jobs = module_drain_buffers(module.module_core_mut());
            module_handle_jobs(rt, jobs);
        }
    }
}

//
// # Helper functions
//

struct ModuleBufferJob {
    module_id: ModuleId,

    out: Vec<(Message, GateId)>,
    loopback: Vec<(Message, SimTime)>,

    ac_event: Option<SimTime>,
}

fn module_drain_buffers(module_core: &mut ModuleCore) -> ModuleBufferJob {
    let enqueue_activity_msg =
        module_core.activity_period != SimTime::ZERO && !module_core.activity_active;

    if enqueue_activity_msg {
        module_core.activity_active = enqueue_activity_msg;
    }

    ModuleBufferJob {
        module_id: module_core.id(),

        out: module_core.out_buffer.drain(..).collect(),
        loopback: module_core.loopback_buffer.drain(..).collect(),

        ac_event: if enqueue_activity_msg {
            Some(module_core.activity_period)
        } else {
            None
        },
    }
}

fn module_handle_jobs<A>(rt: &mut Runtime<NetworkRuntime<A>>, job: ModuleBufferJob) {
    let ModuleBufferJob {
        module_id,
        out,
        loopback,
        ac_event,
    } = job;

    // Send gate events from the 'send' method calls
    for (msg, gate_id) in out {
        rt.add_event(
            NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                gate_id,
                message: ManuallyDrop::new(msg),
                handled: false,
            }),
            SimTime::now(),
        )
    }

    // Send loopback events from 'scheduleAt'
    for (msg, time) in loopback {
        rt.add_event(
            NetEvents::HandleMessageEvent(HandleMessageEvent {
                module_id,
                message: ManuallyDrop::new(msg),
                handled: false,
            }),
            time,
        )
    }

    // initalily
    // call activity
    if let Some(ac_event) = ac_event {
        rt.add_event_in(
            NetEvents::CoroutineMessageEvent(CoroutineMessageEvent { module_id }),
            ac_event,
        )
    }
}
