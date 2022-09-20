use crate::net::message::{
    TYP_IO_TICK, TYP_RESTART, TYP_TCP_CONNECT, TYP_TCP_CONNECT_TIMEOUT, TYP_TCP_PACKET,
    TYP_UDP_PACKET, TYP_WAKEUP,
};
use crate::net::{Message, Module};
use crate::time::SimTime;
use async_trait::async_trait;

pub(crate) mod ext;
use ext::WaitingMessage;

// pub(crate) const RT_UDP: MessageKind = 43;
// pub(crate) const RT_TCP_CONNECT: MessageKind = 44;
// pub(crate) const RT_TCP_CONNECT_TIMEOUT: MessageKind = 45;
// pub(crate) const RT_TCP_PACKET: MessageKind = 46;

///
/// A set of user defined functions for customizing the behaviour
/// of an asynchronous module.
///
/// This trait is just a async version of [`Module`](crate::net::Module).
/// Note that this implementation used [`async_trait`] to provide function
/// signatures.
///
#[async_trait]
pub trait AsyncModule: Send {
    /// Creates a new instance of Self.
    fn new() -> Self
    where
        Self: Sized;

    /// Resets the custom state after shutdown.
    fn reset(&mut self) {}

    ///
    /// A message handler for receiving events, user defined.
    ///
    /// # Note
    ///
    /// The function may block beyond the evaluation of the current event.
    /// If that happens, other messages that will be received will be queued
    /// until the evaluation of this event has concluded. For non-blocking
    /// event execution use [tokio::spawn].
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
    /// # fn new() -> Self { todo!() }
    ///     /* ... */    
    ///
    ///     async fn handle_message(&mut self, msg: Message) {
    ///         println!("Received {:?}", msg);
    ///     }
    /// }
    /// ```
    async fn handle_message(&mut self, _msg: Message) {}

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
    /// # fn new() -> Self { todo!() }
    ///     /* ... */    
    ///
    ///     async fn handle_message(&mut self, _: Message) {
    ///         // ...
    ///     }
    ///
    ///     async fn at_sim_start(&mut self, _stage: usize) {
    ///         self.config = fetch_config(module_id()).await;
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
    fn __manage_intents(&mut self, intents: Vec<tokio::sim::net::IOIntent>) {
        for intent in intents {
            use tokio::sim::net::IOIntent;
            match intent {
                IOIntent::UdpSendPacket(pkt) => {
                    log::info!("Sending captured UDP packet: {:?}", pkt);
                    super::send(
                        Message::new()
                            // .kind(RT_UDP)
                            .typ(TYP_UDP_PACKET)
                            .dest(pkt.dest_addr)
                            .content(pkt)
                            .build(),
                        "out",
                    );
                }
                IOIntent::TcpConnect(pkt) => {
                    log::info!("Sending captured TCP connect: {:?}", pkt);
                    super::send(
                        Message::new()
                            // .kind(RT_TCP_CONNECT)
                            .typ(TYP_TCP_CONNECT)
                            .dest(pkt.dest())
                            .content(pkt)
                            .build(),
                        "out",
                    );
                }
                IOIntent::TcpSendPacket(pkt, delay) => {
                    log::info!("Sending captured TCP packet: {:?}", pkt);
                    super::send_in(
                        Message::new()
                            // .kind(RT_TCP_PACKET)
                            .typ(TYP_TCP_PACKET)
                            .dest(pkt.dest_addr)
                            .content(pkt)
                            .build(),
                        "out",
                        delay,
                    );
                }
                IOIntent::TcpConnectTimeout(pkt, timeout) => {
                    log::info!("Scheduling TCP Connect Timeout: {:?} in {:?}", pkt, timeout);
                    super::schedule_in(
                        Message::new()
                            // .kind(RT_TCP_CONNECT_TIMEOUT)
                            .typ(TYP_TCP_CONNECT_TIMEOUT)
                            .dest(pkt.src())
                            .content(pkt)
                            .build(),
                        timeout,
                    );
                }
                IOIntent::IoTick(wakeup_time) => {
                    log::info!("Scheduling IO Tick at {}", wakeup_time.as_millis());
                    super::schedule_at(Message::new().typ(TYP_IO_TICK).build(), wakeup_time);
                }
                _ => {
                    log::warn!("Unkown Intent");
                }
            }
        }
    }
}

impl<T> Module for T
where
    T: 'static + AsyncModule,
{
    fn new() -> Self
    where
        Self: Sized,
    {
        <T as AsyncModule>::new()
    }

    fn reset(&mut self) {
        #[cfg(not(feature = "async-sharedrt"))]
        super::async_ctx_reset();

        <T as AsyncModule>::reset(self);
    }

    fn handle_message(&mut self, msg: Message) {
        // (1) Fetch the runtime and initial the time context.
        if let Some(rt) = super::async_get_rt() {
            let guard = rt.enter_context(super::async_take_sim_ctx());

            // (2) Poll time time events before excuting
            rt.poll_time_events();

            let typ_processed = match msg.header().typ {
                TYP_WAKEUP => {
                    log::trace!("Wakeup received");
                    Ok(())
                }
                TYP_RESTART => {
                    log::trace!("Module restart complete");
                    Ok(())
                }
                TYP_IO_TICK => {
                    log::trace!("IO tick received");
                    rt.io_tick();
                    Ok(())
                }
                TYP_UDP_PACKET => {
                    use tokio::sim::net::UdpMessage;
                    let (msg, header) = msg.cast::<UdpMessage>();

                    rt.process_udp(msg)
                        .map_err(|msg| Message::new().content(msg).header(header).build())
                }
                TYP_TCP_CONNECT => {
                    use tokio::sim::net::TcpConnectMessage;
                    let (msg, header) = msg.cast::<TcpConnectMessage>();

                    rt.process_tcp_connect(msg)
                        .map_err(|msg| Message::new().content(msg).header(header).build())
                }
                TYP_TCP_CONNECT_TIMEOUT => {
                    use tokio::sim::net::TcpConnectMessage;
                    let (msg, header) = msg.cast::<TcpConnectMessage>();

                    rt.process_tcp_connect_timeout(msg)
                        .map_err(|msg| Message::new().content(msg).header(header).build())
                }
                TYP_TCP_PACKET => {
                    use tokio::sim::net::TcpMessage;
                    let (msg, header) = msg.cast::<TcpMessage>();

                    rt.process_tcp_packet(msg)
                        .map_err(|msg| Message::new().content(msg).header(header).build())
                }
                _ => Err(msg),
            };

            if let Err(msg) = typ_processed {
                super::async_wait_queue_tx_send(WaitingMessage {
                    msg,
                    time: SimTime::now(),
                })
                .expect("Failed to forward message to 'handle_message'");
                // self.async_ext
                //     .wait_queue_tx
                //     .send(WaitingMessage {
                //         msg,
                //         time: SimTime::now(),
                //     })
                //     .expect("Failed to forward message to 'handle_message'")
            }

            rt.poll_until_idle();

            if let Some(next_time) = rt.next_time_poll() {
                super::schedule_at(Message::new().typ(TYP_WAKEUP).build(), next_time);
            }

            self.__manage_intents(rt.yield_intents());

            // (1) Suspend the time context
            super::async_leave_sim_ctx(guard.leave());
        }
    }

    fn at_sim_start(&mut self, stage: usize) {
        // time is 0
        if let Some(rt) = super::async_get_rt() {
            let guard = rt.enter_context(super::async_take_sim_ctx());
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

                    let mut rx = super::async_wait_queue_rx_take().expect("We have been robbed");

                    super::async_set_wait_queue_join(rt.spawn(async move {
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

                    let mut srx =
                        super::async_sim_start_rx_take().expect("We have been robbed at sim start");

                    super::async_set_sim_start_join(rt.spawn(async move {
                        while let Some(stage) = srx.recv().await {
                            if stage == usize::MAX {
                                srx.close();
                                break;
                            }
                            <T as AsyncModule>::at_sim_start(self_ref, stage).await;
                        }
                    }));
                }

                super::async_sim_start_tx_send(stage).expect("Failed to send to unbounded sender");
            }
            rt.poll_until_idle();

            if let Some(next_time) = rt.next_time_poll() {
                super::schedule_at(Message::new().typ(TYP_WAKEUP).build(), next_time);
            }
            self.__manage_intents(rt.yield_intents());

            super::async_leave_sim_ctx(guard.leave());
        }
    }

    fn finish_sim_start(&mut self) {
        if let Some(rt) = super::async_get_rt() {
            let guard = rt.enter_context(super::async_take_sim_ctx());

            rt.poll_time_events();
            {
                super::async_sim_start_tx_send(usize::MAX)
                    .expect("Failed to send close signal to sim_start_task");
            }
            rt.poll_until_idle();

            // The join must succeed else saftey invariant cannot be archived.
            rt.block_or_idle_on(super::async_sim_start_join_take().expect("Crime"))
                .expect("Join Idle")
                .expect("Join Error");

            if let Some(next_time) = rt.next_time_poll() {
                super::schedule_at(Message::new().typ(TYP_WAKEUP).build(), next_time);
            }
            self.__manage_intents(rt.yield_intents());

            super::async_leave_sim_ctx(guard.leave());
        }
    }

    fn at_sim_end(&mut self) {
        if let Some(rt) = super::async_get_rt() {
            let guard = rt.enter_context(super::async_take_sim_ctx());
            rt.poll_time_events();
            {
                // SAFTEY:
                // Sim end means only this function will be executed before drop
                // thus 'static can be assumed.
                let self_ptr: *mut T = self;
                let self_ref: &'static mut T = unsafe { &mut *self_ptr };

                super::async_sim_end_join_set(rt.spawn(<T as AsyncModule>::at_sim_end(self_ref)));
            }
            rt.poll_until_idle();

            // No time event enqueue needed, wont be resolved either way

            super::async_leave_sim_ctx(guard.leave());
        }
    }

    fn finish_sim_end(&mut self) {
        if let Some(rt) = super::async_get_rt() {
            let guard = rt.enter_context(super::async_take_sim_ctx());
            rt.poll_time_events();
            rt.poll_until_idle();

            rt.block_or_idle_on(super::async_sim_end_join_take().expect("Theif"))
                .expect("Join Idle")
                .expect("Join Error");

            // No time event enqueue needed, wont be resolved either way

            super::async_leave_sim_ctx(guard.leave());
        }
    }

    fn num_sim_start_stages(&self) -> usize {
        // Needs at least one sim_start stage to setup the recv handle
        <T as AsyncModule>::num_sim_start_stages(self).min(1)
    }
}
