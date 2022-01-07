use crate::core::*;
use crate::create_event_set;
use crate::net::*;

use lazy_static::__Deref;
use log::{error, info, warn};
use std::mem::ManuallyDrop;

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
    modules: Vec<Box<dyn Module>>,

    ///
    /// The set of channels used to connect module. This will NOT include direct connections
    /// which do not contain any delay, thus are bound to no channel.
    ///
    channels: Vec<Channel>,

    /// A buffer to store all gates
    gate_buffer: GateBuffer,

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

            gate_buffer: GateBuffer::new(),

            inner,
        }
    }

    ///
    /// Registers a boxed module and adds it to the module set.
    /// Returns a mutable refernce to the boxed module.
    /// This reference should be short lived since it blocks any other reference to self.
    ///
    pub fn create_module(&mut self, module: Box<dyn Module>) -> &mut Box<dyn Module> {
        let insert_at = match self.modules.binary_search_by_key(&module.id(), |m| m.id()) {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        self.modules.insert(insert_at, module);
        &mut self.modules[insert_at]
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
    /// Retrieves module by id. This is more efficient that the usual
    /// 'module_mut' because ids a sorted so binary seach can be used.
    ///
    pub fn module_by_id(&self, module_id: ModuleId) -> Option<&dyn Module> {
        let pos = match self.modules.binary_search_by_key(&module_id, |m| m.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(self.modules[pos].deref())
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
    /// Retrieves module mutably by id. This is more efficient that the usual
    /// 'module_mut' because ids a sorted so binary seach can be used.
    ///
    pub fn module_mut_by_id(&mut self, module_id: ModuleId) -> Option<&mut Box<dyn Module>> {
        let pos = match self.modules.binary_search_by_key(&module_id, |m| m.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&mut self.modules[pos])
    }

    ///
    /// Registers a channel with a non-null delay.
    ///
    pub fn create_channel(&mut self, metrics: ChannelMetrics) -> ChannelId {
        let channel = Channel::new(metrics);
        let insert_at = match self
            .channels
            .binary_search_by_key(&channel.id(), |c| c.id())
        {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        self.channels.insert(insert_at, channel);
        self.channels[insert_at].id()
    }

    ///
    /// Retrieves a channel by id.
    ///
    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        let pos = match self.channels.binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&self.channels[pos])
    }

    ///
    /// Retrieves a channel by id mutabliy.
    ///
    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        let pos = match self.channels.binary_search_by_key(&id, |c| c.id()) {
            Ok(pos) => pos,
            Err(_) => return None,
        };

        Some(&mut self.channels[pos])
    }

    pub fn create_gate(&mut self, gate: Gate) -> GateRef {
        self.gate_buffer.insert(gate)
    }

    ///
    /// Retrieves a gate by id from.
    /// This operations should only be done if absuloutly nessecary since it is
    /// expensive, bc gates are stored in their respecitve owner modules.
    ///
    pub fn gate(&self, id: GateId) -> Option<&Gate> {
        self.gate_buffer.gate(id)
    }

    ///
    /// Retrieves a target gate of a gate chain.
    ///
    pub fn gate_dest(&self, source_id: GateId) -> Option<&Gate> {
        self.gate_buffer.gate_dest(source_id)
    }

    ///
    /// Locks the buffer to that no new gates can be created-
    ///
    pub fn finish_building(&mut self) {
        #[cfg(feature = "staticgates")]
        self.gate_buffer.lock()
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

    fn at_simulation_start(rt: &mut Runtime<Self>) {
        // Add inital event
        rt.add_event(NetEvents::SimStartNotif(SimStartNotif()), SimTime::now());
    }
}

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
            channel.set_busy(false);
        }
    }
}

pub struct SimStartNotif();

impl<A> Event<NetworkRuntime<A>> for SimStartNotif {
    fn handle(self, rt: &mut Runtime<NetworkRuntime<A>>) {
        // This is a explicit for loop to prevent borrow rt only in the inner block
        // allowing preemtive dropping of 'module' so that rt can be used in
        // 'module_handle_jobs'.
        for i in 0..rt.app.modules.len() {
            let module = &mut rt.app.modules[i];
            info!(
                target: &format!("Module {}", module.str()),
                "Calling at_simulation_start."
            );
            module.at_simulation_start();

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
