use crate::net::message::{TYP_RESTART, TYP_WAKEUP};
use crate::net::{Message, MessageKind, Module, Packet, StaticModuleCore};
use crate::time::SimTime;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod handle;
pub use handle::*;

pub(crate) mod ext;
use ext::WaitingMessage;

pub(crate) const RT_UDP: MessageKind = 43;
pub(crate) const RT_TCP_CONNECT: MessageKind = 44;
pub(crate) const RT_TCP_CONNECT_TIMEOUT: MessageKind = 45;
pub(crate) const RT_TCP_PACKET: MessageKind = 46;

///
/// A set of user defined functions for customizing the behaviour
/// of an asynchronous module.
///
/// This trait is just a async version of [Module](crate::net::Module).
/// Note that this implementation used [async_trait] to provide function
/// signatures.
///
#[async_trait]
pub trait AsyncModule: StaticModuleCore + Send {
    ///
    /// A message handler for receiving events, user defined.
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::prelude::*;
    /// use async_trait::async_trait;
    ///
    /// #[NdlModule]
    /// struct MyAsyncModule {
    ///     prop_1: f64,
    ///     prop_2: String,
    /// }
    ///
    /// #[async_trait]
    /// impl AsyncModule for MyAsyncModule {
    ///     async fn handle_message(&mut self, msg: Message) {
    ///         let (pkt, meta) = msg.cast::<Packet>();
    ///         println!("Received {:?} with metadata {:?}", pkt, meta);
    ///     }
    /// }
    /// ```
    async fn handle_message(&mut self, _msg: Message) {}

    ///
    /// A periodic activity manager that is activated if [ModuleCore::enable_activity] is
    /// set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::prelude::*;
    /// use async_trait::async_trait;
    ///
    /// # fn is_good_packet(pkt: Packet) -> bool { true }
    ///
    /// #[NdlModule]
    /// struct MyChannelProbe {
    ///     goodput: u64,
    ///     throughput: u64,
    ///
    ///     metrics: des::tokio::sync::mpsc::Sender<f64>,
    /// }
    ///
    /// #[async_trait]
    /// impl AsyncModule for MyChannelProbe {
    ///     async fn handle_message(&mut self, msg: Message) {
    ///         let (pkt, _meta) = msg.cast::<Packet>();
    ///         self.throughput += 1;        
    ///         if is_good_packet(pkt) {
    ///             self.goodput += 1;
    ///         }
    ///     }
    ///
    ///     async fn activity(&mut self) {
    ///         let rate = (self.goodput as f64) / (self.throughput as f64);
    ///         self.goodput = 0;
    ///         self.throughput = 0;
    ///         self.metrics.send(rate).await.expect("Failed to send");
    ///     }
    /// }
    /// ```
    ///
    async fn activity(&mut self) {}

    ///
    /// A function that is run at the start of each simulation, for each module.
    /// The order in which modules are called is not guranteed but the stage numbers are.
    /// That means that all stage-0 calls for all modules happen before the first (if any) stage-1 calls.
    /// Generaly speaking, all stage-i calls finish before the first stage-i+1 call.
    ///
    /// # Note
    ///
    /// The user must ensure that all calls of `at_sim_start` will terminate at last
    /// once all stages of at_sim_start of all modules have been called.
    /// The stages will be executed in order.
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::prelude::*;
    /// use async_trait::async_trait;
    ///
    /// # type Config = ();
    /// async fn fetch_config(id: ModuleId) -> Config {
    ///     // ...
    /// }
    ///
    /// #[NdlModule]
    /// struct MyModule {
    ///     config: Config,
    ///     records: Vec<f64>,
    /// }
    ///
    /// #[async_trait]
    /// impl AsyncModule for MyModule {
    ///     async fn handle_message(&mut self, _: Message) {
    ///         // ...
    ///     }
    ///
    ///     async fn at_sim_start(&mut self, _stage: usize) {
    ///         self.config = fetch_config(self.id()).await;
    ///         self.records.clear();
    ///     }
    /// }
    ///
    /// ```
    async fn at_sim_start(&mut self, _stage: usize) {}

    ///
    /// Module shutdown and restart is not supported with the feature 'asnyc-sharedrt'.
    ///
    #[cfg(feature = "async-sharedrt")]
    #[deprecated(
        note = "Module shutdown and restart is not supported with the feature 'asnyc-sharedrt'"
    )]
    fn at_restart(&mut self) {}

    ///
    /// A function that is called once the module restarts,
    /// after using [shutdown](super::core::ModuleCore::shutdown).
    /// This means that all async elements have been pruged,
    /// but the local state of `self` is not yet reset.
    ///
    /// Use this function to reset the local state of nessecary.
    ///
    #[cfg(not(feature = "async-sharedrt"))]
    fn at_restart(&mut self) {}

    ///
    /// A function that is called once the simulation has terminated.
    /// Any event created by this function will be ignored.
    ///
    async fn at_sim_end(&mut self) {}

    ///
    /// A function that is called if the parameterst of the simulation
    /// enviroment was changed
    ///
    async fn handle_par_change(&mut self) {}

    ///
    /// A function that returns the number of required startup stages
    /// of a module.
    ///
    fn num_sim_start_stages(&self) -> usize {
        1
    }

    #[doc(hidden)]
    #[cfg(feature = "async-sharedrt")]
    fn __get_rt(&self) -> Option<Arc<Runtime>> {
        Some(Arc::clone(&self.globals().runtime))
    }

    #[doc(hidden)]
    #[cfg(not(feature = "async-sharedrt"))]
    fn __get_rt(&self) -> Option<Arc<Runtime>> {
        Some(Arc::clone(self.module_core().async_ext.rt.as_ref()?))
    }

    #[doc(hidden)]
    fn __manage_intents(&mut self, intents: Vec<tokio::sim::net::IOIntent>) {
        for intent in intents {
            use tokio::sim::net::IOIntent;
            match intent {
                IOIntent::UdpSendPacket(pkt) => {
                    log::info!("Sending captured UDP packet: {:?}", pkt);
                    self.send(
                        Packet::new()
                            .kind(RT_UDP)
                            .dest_addr(pkt.dest_addr)
                            .content(pkt)
                            .build(),
                        "out",
                    );
                }
                IOIntent::TcpConnect(pkt) => {
                    log::info!("Sending captured TCP connect: {:?}", pkt);
                    self.send(
                        Packet::new()
                            .kind(RT_TCP_CONNECT)
                            .dest_addr(pkt.dest())
                            .content(pkt)
                            .build(),
                        "out",
                    )
                }
                IOIntent::TcpConnectTimeout(pkt, timeout) => {
                    log::info!("Scheduling TCP Connect Timeout: {:?} in {:?}", pkt, timeout);
                    self.schedule_in(
                        Packet::new()
                            .kind(RT_TCP_CONNECT_TIMEOUT)
                            // .dest_addr(pkt.)
                            .content(pkt)
                            .build(),
                        timeout,
                    )
                }
                IOIntent::TcpSendPacket(pkt) => {
                    log::info!("Sending captured TCP packet: {:?}", pkt);
                    self.send(
                        Packet::new()
                            .kind(RT_TCP_PACKET)
                            .dest_addr(pkt.dest_addr)
                            .content(pkt)
                            .build(),
                        "out",
                    )
                }
                _ => {
                    log::warn!("Unkown Intent")
                }
            }
        }
    }
}

impl<T> Module for T
where
    T: 'static + AsyncModule,
{
    fn handle_message(&mut self, msg: Message) {
        // (0) Check meta messaeg
        #[cfg(not(feature = "async-sharedrt"))]
        if msg.meta().typ == TYP_RESTART {
            self.async_ext.reset();
            self.at_restart();

            // Do sim start procedure
            let stages = <Self as Module>::num_sim_start_stages(self);
            for stage in 0..stages {
                <Self as Module>::at_sim_start(self, stage);
            }
            <Self as Module>::finish_sim_start(self);
        }

        // (1) Fetch the runtime and initial the time context.
        if let Some(rt) = self.__get_rt() {
            let guard = rt.enter_context(self.async_ext.ctx.take().unwrap());

            // (2) Poll time time events before excuting
            rt.poll_time_events();

            match msg.meta().typ {
                0 => match msg.meta().kind {
                    RT_UDP => {
                        use tokio::sim::net::UdpMessage;
                        let msg = msg.as_packet();
                        let (msg, _) = msg.cast::<UdpMessage>();

                        rt.process_udp(msg);
                    }
                    RT_TCP_CONNECT => {
                        use tokio::sim::net::TcpConnectMessage;
                        let msg = msg.as_packet();
                        let (msg, _) = msg.cast::<TcpConnectMessage>();

                        rt.process_tcp_connect(msg);
                    }
                    RT_TCP_CONNECT_TIMEOUT => {
                        use tokio::sim::net::TcpConnectMessage;
                        let msg = msg.as_packet();
                        let (msg, _) = msg.cast::<TcpConnectMessage>();

                        rt.process_tcp_connect_timeout(msg);
                    }
                    RT_TCP_PACKET => {
                        use tokio::sim::net::TcpMessage;
                        let msg = msg.as_packet();
                        let (msg, _) = msg.cast::<TcpMessage>();

                        rt.process_tcp_packet(msg);
                    }
                    _ => {
                        self.async_ext
                            .wait_queue_tx
                            .send(WaitingMessage {
                                msg,
                                time: SimTime::now(),
                            })
                            .expect("Failed to send to unbounded channel");
                    }
                },
                TYP_WAKEUP => {}
                TYP_RESTART => {
                    log::trace!("Module restart complete")
                }
                _ => unimplemented!(""),
            }

            rt.poll_until_idle();

            if let Some(next_time) = rt.next_time_poll() {
                self.schedule_at(Message::new().typ(TYP_WAKEUP).build(), next_time);
            }

            self.__manage_intents(rt.yield_intents());

            // (1) Suspend the time context
            self.async_ext.ctx = Some(guard.leave());
        }
    }

    fn activity(&mut self) {
        if let Some(rt) = self.__get_rt() {
            let guard = rt.enter_context(self.async_ext.ctx.take().unwrap());

            rt.poll_time_events();
            {
                let self_ptr: *mut T = self;
                let self_ref: &'static mut T = unsafe { &mut *self_ptr };

                let join = rt.spawn(<T as AsyncModule>::activity(self_ref));
                let _result = rt.block_or_idle_on(join);
            }
            rt.poll_until_idle();

            if let Some(next_time) = rt.next_time_poll() {
                self.schedule_at(Message::new().typ(TYP_WAKEUP).build(), next_time);
            }

            self.__manage_intents(rt.yield_intents());

            self.async_ext.ctx = Some(guard.leave());
        }
    }

    fn at_sim_start(&mut self, stage: usize) {
        // time is 0
        if let Some(rt) = self.__get_rt() {
            let guard = rt.enter_context(self.async_ext.ctx.take().unwrap());
            rt.poll_time_events();
            {
                // # Setup message receive handle.
                if stage == 0 {
                    // SAFTEY:
                    // We can guarntee the validity of the pointer:
                    // 1) The module is pinned while the simulation is running.
                    // 2) The module is not dropped while the simulation is running.
                    // 3) While we may create mutiple &mut T, handle_message is never run fully
                    //    async (current thread runtime) and mutiple calls of `handle_messsage`
                    //    wont overlap, since the queue rx synchronises and delays them.
                    // 4) References to at_sim_start have been droped since all futures of at_sim_start
                    //    must be resoved before event 1
                    //
                    // TODO: Sync with activity()
                    let self_ref: &'static mut T = {
                        let ptr: *mut T = self;
                        unsafe { &mut *ptr }
                    };

                    let mut rx = self
                        .async_ext
                        .wait_queue_rx
                        .take()
                        .expect("We have been robbed");

                    self.async_ext.wait_queue_join = Some(rt.spawn(async move {
                        while let Some(wmsg) = rx.recv().await {
                            let WaitingMessage { msg, .. } = wmsg;
                            <T as AsyncModule>::handle_message(self_ref, msg).await;
                        }
                    }));
                }

                // # Setup Sim-Start Task
                if stage == 0 {
                    // SAFTEY:
                    // SimStart will complete before event id 1. thus this is quasai sync
                    let self_ref: &'static mut T = {
                        let ptr: *mut T = self;
                        unsafe { &mut *ptr }
                    };

                    let mut srx = self
                        .async_ext
                        .sim_start_rx
                        .take()
                        .expect("We have been robbed at sim start");

                    self.async_ext.sim_start_join = Some(rt.spawn(async move {
                        while let Some(stage) = srx.recv().await {
                            if stage == usize::MAX {
                                srx.close();
                                break;
                            }
                            <T as AsyncModule>::at_sim_start(self_ref, stage).await;
                        }
                    }));
                }

                self.async_ext
                    .sim_start_tx
                    .send(stage)
                    .expect("Failed to send to unbounded sender");
            }
            rt.poll_until_idle();

            if let Some(next_time) = rt.next_time_poll() {
                self.schedule_at(Message::new().typ(TYP_WAKEUP).build(), next_time);
            }
            self.__manage_intents(rt.yield_intents());

            self.async_ext.ctx = Some(guard.leave());
        }
    }

    fn finish_sim_start(&mut self) {
        if let Some(rt) = self.__get_rt() {
            let guard = rt.enter_context(self.async_ext.ctx.take().unwrap());

            rt.poll_time_events();
            {
                self.async_ext
                    .sim_start_tx
                    .send(usize::MAX)
                    .expect("Failed to send close signal to sim_start_task");
            }
            rt.poll_until_idle();

            // The join must succeed else saftey invariant cannot be archived.
            rt.block_or_idle_on(self.async_ext.sim_start_join.take().expect("Crime"))
                .expect("Join Idle")
                .expect("Join Error");

            if let Some(next_time) = rt.next_time_poll() {
                self.schedule_at(Message::new().typ(TYP_WAKEUP).build(), next_time);
            }
            self.__manage_intents(rt.yield_intents());

            self.async_ext.ctx = Some(guard.leave());
        }
    }

    fn at_sim_end(&mut self) {
        if let Some(rt) = self.__get_rt() {
            let guard = rt.enter_context(self.async_ext.ctx.take().unwrap());
            rt.poll_time_events();
            {
                // SAFTEY:
                // Sim end means only this function will be executed before drop
                // thus 'static can be assumed.
                let self_ptr: *mut T = self;
                let self_ref: &'static mut T = unsafe { &mut *self_ptr };

                self.async_ext.sim_end_join =
                    Some(rt.spawn(<T as AsyncModule>::at_sim_end(self_ref)));
            }
            rt.poll_until_idle();

            // No time event enqueue needed, wont be resolved either way

            self.async_ext.ctx = Some(guard.leave());
        }
    }

    fn finish_sim_end(&mut self) {
        if let Some(rt) = self.__get_rt() {
            let guard = rt.enter_context(self.async_ext.ctx.take().unwrap());
            rt.poll_time_events();
            rt.poll_until_idle();

            rt.block_or_idle_on(self.async_ext.sim_end_join.take().expect("Theif"))
                .expect("Join Idle")
                .expect("Join Error");

            // No time event enqueue needed, wont be resolved either way

            self.async_ext.ctx = Some(guard.leave());
        }
    }

    fn num_sim_start_stages(&self) -> usize {
        // Needs at least one sim_start stage to setup the recv handle
        <T as AsyncModule>::num_sim_start_stages(self).min(1)
    }
}
