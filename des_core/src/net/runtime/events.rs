use log::{info, warn};
use std::mem::ManuallyDrop;
use std::ops::Deref;

use crate::core::*;
use crate::create_event_set;
use crate::net::*;

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
    pub message: ManuallyDrop<Message>,
    pub handled: bool,
}

impl<A> Event<NetworkRuntime<A>> for MessageAtGateEvent {
    fn handle(mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        let ptr: *const Message = self.message.deref();
        let mut message = unsafe { std::ptr::read(ptr) };
        message.meta.last_gate = self.gate.id();

        self.handled = true;

        //
        // Iterate through gates until:
        // a) a final gate with no next_gate was found, indicating a handle_module_call
        // b) a delay gate was found, apply the delay and recall in a new event.
        //
        let mut current_gate = &self.gate;
        while let Some(next_gate) = current_gate.next_gate() {
            // A next gate exists.
            // redirect to next channel

            info!(
                target: &format!("Gate #{} ({})", current_gate.id(), current_gate.name()),
                "Forwarding message [{}] to next gate #{} delyed: {}",
                message.str(),
                next_gate.id(),
                current_gate.channel().is_some()
            );

            match current_gate.channel() {
                Some(channel) => {
                    let mut channel = channel.clone();

                    // Channel delayed connection
                    // SAFTY:
                    // The rng and random number generator dont interfere so this operation can
                    // be considered safe. Make sure this ref is only used in conjunction with the channel.
                    let rng_ref = unsafe { &mut (*rt.rng()) };

                    if channel.is_busy() {
                        warn!(
                            target: &format!("Gate #{}", current_gate.id()),
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
                            gate: next_gate.clone(),
                            message: ManuallyDrop::new(message),
                            handled: false,
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
        assert!(current_gate.next_gate().is_none());

        info!(
            target: &format!("Gate #{} ({})", current_gate.id(), current_gate.name()),
            "Forwarding message [{}] to module #{}",
            message.str(),
            current_gate.module().id()
        );

        let module = current_gate.module().clone();
        rt.add_event(
            NetEvents::HandleMessageEvent(HandleMessageEvent {
                module,
                message: ManuallyDrop::new(message),
                handled: false,
            }),
            SimTime::now(),
        );
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
    pub module: ModuleRef,
    pub message: ManuallyDrop<Message>,
    pub handled: bool,
}

impl<A> Event<NetworkRuntime<A>> for HandleMessageEvent {
    fn handle(mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        let ptr: *const Message = self.message.deref();
        let mut message = unsafe { std::ptr::read(ptr) };
        message.meta.receiver_module_id = self.module.id();

        info!(
            target: &format!("Module {}", self.module.str()),
            "Handling message {:?}",
            message.str()
        );

        self.module.handle_message(message);

        let job = module_drain_buffers(self.module.clone());
        module_handle_jobs(rt, job);

        self.handled = true;
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
    module: ModuleRef,
}

impl<A> Event<NetworkRuntime<A>> for CoroutineMessageEvent {
    fn handle(mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        let dur = self.module.module_core().activity_period;
        // This message can only occure *after* the activity is
        // fully initalized.
        // It can be the case that this is wrong .. if handle_message at invalidated activity
        // but a event still remains in queue.
        if dur != SimTime::ZERO && self.module.module_core().activity_active {
            self.module.activity();

            let still_active = self.module.module_core().activity_active;

            // This call will only use up out buffers and loopback buffers
            // not schedule a new Coroutine since either:
            // - state remains stable since allready init
            // - activity deactivated.
            let jobs = module_drain_buffers(self.module.clone());
            module_handle_jobs(rt, jobs);

            if still_active {
                rt.add_event_in(
                    NetEvents::CoroutineMessageEvent(CoroutineMessageEvent {
                        module: self.module.clone(),
                    }),
                    dur,
                )
            }
        }
    }
}

pub struct ChannelUnbusyNotif {
    channel: ChannelRef,
}

impl<A> Event<NetworkRuntime<A>> for ChannelUnbusyNotif {
    fn handle(mut self, _rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        self.channel.unbusy()
    }
}

pub struct SimStartNotif();

impl<A> Event<NetworkRuntime<A>> for SimStartNotif {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        // This is a explicit for loop to prevent borrow rt only in the inner block
        // allowing preemtive dropping of 'module' so that rt can be used in
        // 'module_handle_jobs'.
        for i in 0..rt.app.modules().len() {
            let mut module = rt.app.modules()[i].clone();
            info!(
                target: &format!("Module {}", module.str()),
                "Calling at_sim_start."
            );
            module.at_sim_start();

            let jobs = module_drain_buffers(module);
            module_handle_jobs(rt, jobs);
        }
    }
}

//
// # Helper functions
//

struct ModuleBufferJob {
    module: ModuleRef,

    out: Vec<(Message, GateRef)>,
    loopback: Vec<(Message, SimTime)>,

    ac_event: Option<SimTime>,
}

fn module_drain_buffers(mut module: ModuleRef) -> ModuleBufferJob {
    let module_core = module.module_core_mut();
    let enqueue_activity_msg =
        module_core.activity_period != SimTime::ZERO && !module_core.activity_active;

    if enqueue_activity_msg {
        module_core.activity_active = enqueue_activity_msg;
    }

    ModuleBufferJob {
        out: module_core.out_buffer.drain(..).collect(),
        loopback: module_core.loopback_buffer.drain(..).collect(),

        ac_event: if enqueue_activity_msg {
            Some(module_core.activity_period)
        } else {
            None
        },

        module,
    }
}

fn module_handle_jobs<A>(rt: &mut Runtime<NetworkRuntime<A>>, job: ModuleBufferJob) {
    let ModuleBufferJob {
        module,
        out,
        loopback,
        ac_event,
    } = job;

    // Send gate events from the 'send' method calls
    for (msg, gate) in out {
        rt.add_event(
            NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                gate,
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
                module: module.clone(),
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
            NetEvents::CoroutineMessageEvent(CoroutineMessageEvent { module }),
            ac_event,
        )
    }
}
