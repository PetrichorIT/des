mod channel;
mod gate;
mod message;
mod module;
mod packet;

mod tests;

use std::{fmt::Debug, mem::ManuallyDrop};

pub use channel::*;
pub use gate::*;
pub use message::*;
pub use module::*;
pub use packet::*;

use crate::{Event, SimTime};
use log::error;

pub struct NetworkRuntime<A> {
    pub modules: Vec<Module>,
    pub channels: Vec<Channel>,

    pub inner: A,
}

impl<A> NetworkRuntime<A> {
    pub fn new(inner: A) -> Self {
        Self {
            modules: Vec::new(),
            channels: Vec::new(),

            inner,
        }
    }

    pub fn module(&self, module_id: ModuleId) -> Option<&Module> {
        self.modules.iter().find(|m| m.id == module_id)
    }

    pub fn module_mut(&mut self, module_id: ModuleId) -> Option<&mut Module> {
        self.modules.iter_mut().find(|m| m.id == module_id)
    }

    pub fn gate(&self, id: GateId) -> Option<&Gate> {
        for module in &self.modules {
            for gate in &module.gates {
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

    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.iter().find(|c| c.id == id)
    }

    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|c| c.id == id)
    }
}

pub struct MessageAtGateEvent<T: MessageBody> {
    gate_id: GateId,
    message: ManuallyDrop<Message<T>>,
    handled: bool,
}

impl<A, T> Event<NetworkRuntime<A>> for MessageAtGateEvent<T>
where
    T: MessageBody + Debug + 'static,
{
    fn handle(&mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        let gate = rt.app.gate(self.gate_id);
        if let Some(gate) = gate {
            let ptr: *const Message<T> = unsafe { &ManuallyDrop::take(&mut self.message) };
            let mut message = unsafe { std::ptr::read(ptr) };
            message.register_hop(gate.id());

            self.handled = true;

            println!("Gate [{}] handling event {:?}", self.gate_id, message);

            if gate.next_gate() == GATE_SELF || gate.next_gate() == GATE_NULL {
                // handle message
                // calulate busy time at last hop
                let id = gate.channel();
                let module = gate.module();

                let channel = rt.app.channel_mut(id).unwrap();
                assert!(!channel.busy);

                let dur = channel.calculate_duration(&message.content);
                let busy = channel.calculate_busy(&message.content);
                channel.busy = true;

                rt.add_event_in(ChannelUnbusyNotif { channel_id: id }, busy);
                rt.add_event_in(
                    HandleMessageEvent {
                        module_id: module,
                        message: ManuallyDrop::new(message),
                    },
                    dur,
                );
            } else {
                // redirect to next channel
                let next_gate = gate.next_gate();

                let channel = rt.app.channel(gate.channel()).unwrap();
                if channel.busy {
                    error!(
                        "Dropping message {:?} pushed onto busy channel {:?}",
                        message, channel
                    );
                    drop(message);
                    return;
                }

                rt.add_event(
                    MessageAtGateEvent {
                        gate_id: next_gate,
                        message: ManuallyDrop::new(message),
                        handled: false,
                    },
                    SimTime::now(),
                )
            }
        } else {
            error!(
                "Message {:?} dropped after gate id {} was not found",
                self.message, self.gate_id
            )
        }
    }
}

impl<T: MessageBody> Drop for MessageAtGateEvent<T> {
    fn drop(&mut self) {
        if !self.handled {
            unsafe { ManuallyDrop::drop(&mut self.message) }
        }
    }
}

pub struct HandleMessageEvent<T: MessageBody> {
    module_id: ModuleId,
    #[allow(dead_code)]
    message: ManuallyDrop<Message<T>>,
}

impl<A, T: MessageBody> Event<NetworkRuntime<A>> for HandleMessageEvent<T> {
    fn handle(&mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        if let Some(_module) = rt.app.module(self.module_id) {
            println!(
                "[Module {}] handle message {:?}",
                self.module_id, self.message
            )
        } else {
            error!("Dropped message for module id {}", self.module_id)
        }
    }
}

impl<T: MessageBody> Drop for HandleMessageEvent<T> {
    fn drop(&mut self) {}
}

pub struct CoroutineMessageEvent {}

pub struct ChannelUnbusyNotif {
    channel_id: ChannelId,
}

impl<A> Event<NetworkRuntime<A>> for ChannelUnbusyNotif {
    fn handle(&mut self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
        if let Some(channel) = rt.app.channels.iter_mut().find(|c| c.id == self.channel_id) {
            channel.busy = false;
        }
    }
}
