mod channel;
mod gate;
mod message;
mod module;
mod packet;

use std::mem::ManuallyDrop;

pub use channel::*;
pub use gate::*;
pub use message::*;
pub use module::*;
pub use packet::*;

use crate::{Event, SimTime};
use log::{error, info, warn};

pub struct NetworkRuntime<A> {
    pub modules: Vec<Box<dyn Module>>,
    pub channels: Vec<Channel>,

    pub inner: A,
}

impl<A> NetworkRuntime<A> {
    pub fn new(inner: A) -> Self {
        Self {
            modules: Vec::new(),
            channels: vec![Channel::INSTANTANEOUS],

            inner,
        }
    }

    pub fn create_module(&mut self, module: Box<dyn Module>) -> &mut Box<dyn Module> {
        self.modules.push(module);
        self.modules.last_mut().unwrap()
    }

    pub fn module(&self, module_id: ModuleId) -> Option<&dyn Module> {
        self.modules
            .iter()
            .find(|m| m.id() == module_id)
            .map(|c| c.as_ref())
    }

    pub fn module_mut(&mut self, module_id: ModuleId) -> Option<&mut Box<dyn Module>> {
        self.modules.iter_mut().find(|m| m.id() == module_id)
    }

    pub fn gate(&self, id: GateId) -> Option<&Gate> {
        for module in &self.modules {
            for gate in module.gates() {
                if gate.id() == id {
                    return Some(gate);
                }
            }
        }

        None
    }

    pub fn gate_dest(&self, source_id: GateId) -> Option<&Gate> {
        let mut gate = self.gate(source_id)?;
        while gate.id() != GATE_SELF {
            gate = self.gate(gate.next_gate())?
        }
        Some(gate)
    }

    pub fn create_channel(&mut self, metrics: ChannelMetrics) -> ChannelId {
        let channel = Channel::new(metrics);
        self.channels.push(channel);
        self.channels.last().unwrap().id()
    }

    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.iter().find(|c| c.id() == id)
    }

    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|c| c.id() == id)
    }
}

pub struct MessageAtGateEvent {
    pub gate_id: GateId,
    pub message: ManuallyDrop<Message>,
    pub handled: bool,
}

impl<A> Event<NetworkRuntime<A>> for MessageAtGateEvent {
    fn handle(&mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        let gate = rt.app.gate(self.gate_id);
        if let Some(gate) = gate {
            let ptr: *const Message = unsafe { &ManuallyDrop::take(&mut self.message) };
            let mut message = unsafe { std::ptr::read(ptr) };
            message.set_last_gate(self.gate_id);

            self.handled = true;

            if gate.next_gate() == GATE_SELF || gate.next_gate() == GATE_NULL {
                info!(
                    target: &format!("Gate #{} ({})", self.gate_id, gate.name()),
                    "Forwarding message [{}] to module #{}",
                    message.str(),
                    gate.module()
                );

                let module = gate.module();
                rt.add_event(
                    HandleMessageEvent {
                        module_id: module,
                        message: ManuallyDrop::new(message),
                        handled: false,
                    },
                    SimTime::now(),
                );
            } else {
                // redirect to next channel
                let next_gate = gate.next_gate();

                info!(
                    target: &format!("Gate #{} ({})", self.gate_id, gate.name()),
                    "Forwarding message [{}] to next gate #{}",
                    message.str(),
                    next_gate
                );

                let next_event_time = if gate.channel() == CHANNEL_NULL {
                    // Direct connection
                    SimTime::now()
                } else {
                    // Channel delayed connection
                    let channel_id = gate.channel();
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

                    let dur = channel.calculate_duration(&message);
                    let busy = channel.calculate_busy(&message);

                    channel.set_busy(true);

                    rt.add_event_in(ChannelUnbusyNotif { channel_id }, busy);

                    SimTime::now() + dur
                };

                rt.add_event(
                    MessageAtGateEvent {
                        gate_id: next_gate,
                        message: ManuallyDrop::new(message),
                        handled: false,
                    },
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
    fn handle(&mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        if let Some(module) = rt.app.module_mut(self.module_id) {
            let ptr: *const Message = unsafe { &ManuallyDrop::take(&mut self.message) };
            let mut message = unsafe { std::ptr::read(ptr) };
            message.set_target_module(module.id());

            self.handled = true;

            info!(
                target: &format!("Module {}", module.str()),
                "Handling message {:?}",
                message.str()
            );

            module.handle_message(message);

            // Send out events
            let out_buffer: Vec<(Message, GateId)> =
                module.module_core_mut().out_buffer.drain(0..).collect();

            // Schedule own wakeups
            let loopback_buffer: Vec<(Message, SimTime)> = module
                .module_core_mut()
                .loopback_buffer
                .drain(0..)
                .collect();

            let enqueue_activity_msg = module.module_core_mut().activity_period != SimTime::ZERO
                && !module.module_core_mut().activity_active;

            for (msg, gate_id) in out_buffer {
                rt.add_event(
                    MessageAtGateEvent {
                        gate_id,
                        message: ManuallyDrop::new(msg),
                        handled: false,
                    },
                    SimTime::now(),
                )
            }

            for (msg, time) in loopback_buffer {
                rt.add_event(
                    HandleMessageEvent {
                        module_id: self.module_id,
                        message: ManuallyDrop::new(msg),
                        handled: false,
                    },
                    time,
                )
            }

            if enqueue_activity_msg {
                rt.add_event(
                    CoroutineMessageEvent {
                        module_id: self.module_id,
                    },
                    SimTime::now(),
                )
            }
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
    fn handle(&mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        if let Some(module) = rt.app.module_mut(self.module_id) {
            let dur = module.module_core().activity_period;
            if dur != SimTime::ZERO {
                module.activity();

                rt.add_event_in(
                    CoroutineMessageEvent {
                        module_id: self.module_id,
                    },
                    dur,
                )
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
    fn handle(&mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        if let Some(channel) = rt
            .app
            .channels
            .iter_mut()
            .find(|c| c.id() == self.channel_id)
        {
            channel.set_busy(false);
        }
    }
}
