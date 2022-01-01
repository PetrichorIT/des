use crate::core::*;
use crate::net::*;

use lazy_static::__Deref;
use std::mem::ManuallyDrop;

use crate::{Event, EventSet, SimTime};
use log::{error, info, warn};

///
/// A runtime application for a module/network oriantated simulation.
///
/// * This type is only available of DES is build with the `"net"` feature.
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub struct NetworkRuntime<A> {
    ///
    /// The set of module used in the network simulation.
    /// All module must be boxed, since they must conform to the [Module] trait.
    ///
    pub modules: Vec<Box<dyn Module>>,

    ///
    /// The set of channels used to connect module. This will NOT include direct connections
    /// which do not contain any delay, thus are bound to no channel.
    ///
    pub channels: Vec<Channel>,

    ///
    /// A inner container for holding user defined global state.
    ///
    pub inner: A,
}

impl<A> NetworkRuntime<A> {
    ///
    /// Creates a new instance by wrapping 'inner' into a empty NetworkRuntime<A>.
    ///
    pub fn new(inner: A) -> Self {
        Self {
            modules: Vec::new(),
            channels: vec![],

            inner,
        }
    }

    ///
    /// Registers a boxed module and adds it to the module set.
    /// Returns a mutable refernce to the boxed module.
    /// This reference should be short lived since it blocks any other reference to self.
    ///
    pub fn create_module(&mut self, module: Box<dyn Module>) -> &mut Box<dyn Module> {
        self.modules.push(module);
        self.modules.last_mut().unwrap()
    }

    ///
    /// Retrieves a module that staisfies the given predicate.
    /// Shortcircuits once such a element is found.
    ///
    pub fn module<P>(&self, predicate: P) -> Option<&dyn Module>
    where
        P: FnMut(&&Box<dyn Module>) -> bool,
    {
        self.modules
            .iter()
            .find(predicate)
            .map(|boxed| boxed.deref())
    }

    ///
    /// Leacy support. Deprecation planned.
    ///
    pub fn module_by_id(&self, module_id: ModuleId) -> Option<&dyn Module> {
        self.module(|m| m.id() == module_id)
    }

    ///
    /// Retrieves a module mutably that staisfies the given predicate.
    /// Shortcircuits once such a element is found.
    ///
    pub fn module_mut<P>(&mut self, predicate: P) -> Option<&mut Box<dyn Module>>
    where
        P: FnMut(&&mut Box<dyn Module>) -> bool,
    {
        self.modules.iter_mut().find(predicate)
    }

    ///
    /// Leacy support. Deprecation planned.
    ///
    pub fn module_mut_by_id(&mut self, module_id: ModuleId) -> Option<&mut Box<dyn Module>> {
        self.module_mut(|m| m.id() == module_id)
    }

    ///
    /// Registers a channel with a non-null delay.
    ///
    pub fn create_channel(&mut self, metrics: ChannelMetrics) -> ChannelId {
        let channel = Channel::new(metrics);
        self.channels.push(channel);
        self.channels.last().unwrap().id()
    }

    ///
    /// Retrieves a channel by id.
    ///
    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.iter().find(|c| c.id() == id)
    }

    ///
    /// Retrieves a channel by id mutabliy.
    ///
    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|c| c.id() == id)
    }

    ///
    /// Retrieves a gate by id from.
    /// This operations should only be done if absuloutly nessecary since it is
    /// expensive, bc gates are stored in their respecitve owner modules.
    ///
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

    ///
    /// Retrieves a target gate of a gate chain.
    ///
    pub fn gate_dest(&self, source_id: GateId) -> Option<&Gate> {
        // TODO: make more efficient variant.
        let mut gate = self.gate(source_id)?;
        while gate.id() != GATE_SELF {
            gate = self.gate(gate.next_gate())?
        }
        Some(gate)
    }

    ///
    /// Drops all modules and channels and only returns the inner value.
    ///
    pub fn finish(self) -> A {
        self.inner
    }
}

impl<A> Application for NetworkRuntime<A> {
    type EventSet = NetEvents;
}

///
/// The event set for a [NetworkRuntime].
///
pub enum NetEvents {
    MessageAtGateEvent(MessageAtGateEvent),
    HandleMessageEvent(HandleMessageEvent),
    CoroutineMessageEvent(CoroutineMessageEvent),
    ChannelUnbusyNotif(ChannelUnbusyNotif),
}

impl<A> EventSet<NetworkRuntime<A>> for NetEvents {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        match self {
            Self::MessageAtGateEvent(event) => event.handle(rt),
            Self::HandleMessageEvent(event) => event.handle(rt),
            Self::CoroutineMessageEvent(event) => event.handle(rt),
            Self::ChannelUnbusyNotif(event) => event.handle(rt),
        }
    }
}

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

                    rt.add_event_in(
                        NetEvents::ChannelUnbusyNotif(ChannelUnbusyNotif { channel_id }),
                        busy,
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
            message.set_target_module(module.id());

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
                    NetEvents::MessageAtGateEvent(MessageAtGateEvent {
                        gate_id,
                        message: ManuallyDrop::new(msg),
                        handled: false,
                    }),
                    SimTime::now(),
                )
            }

            for (msg, time) in loopback_buffer {
                rt.add_event(
                    NetEvents::HandleMessageEvent(HandleMessageEvent {
                        module_id: self.module_id,
                        message: ManuallyDrop::new(msg),
                        handled: false,
                    }),
                    time,
                )
            }

            if enqueue_activity_msg {
                rt.add_event(
                    NetEvents::CoroutineMessageEvent(CoroutineMessageEvent {
                        module_id: self.module_id,
                    }),
                    SimTime::now(),
                )
            }

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
            if dur != SimTime::ZERO {
                module.activity();

                rt.add_event_in(
                    NetEvents::CoroutineMessageEvent(CoroutineMessageEvent {
                        module_id: self.module_id,
                    }),
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
    fn handle(self, rt: &mut crate::Runtime<NetworkRuntime<A>>) {
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
