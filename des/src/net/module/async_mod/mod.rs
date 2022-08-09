use crate::net::{Message, MessageKind, Module, StaticModuleCore};
use crate::time::SimTime;
use async_trait::async_trait;
use std::sync::Arc;

mod handle;
pub(crate) use handle::*;

pub(crate) mod ext;
use ext::WaitingMessage;

pub(crate) const RT_TIME_WAKEUP: MessageKind = 42;

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
    async fn handle_message(&mut self, msg: Message);

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
}

impl<T> Module for T
where
    T: 'static + AsyncModule,
{
    fn handle_message(&mut self, msg: Message) {
        // (1) Fetch the runtime and initial the time context.
        let rt = Arc::clone(&self.globals().runtime);
        let guard = rt.enter_time_context(self.async_ext.time_context.take().unwrap());

        // (2) Poll time time events before excuting
        rt.poll_time_events();
        if msg.meta().kind != RT_TIME_WAKEUP {
            self.async_ext
                .wait_queue_tx
                .send(WaitingMessage {
                    msg,
                    time: SimTime::now(),
                })
                .expect("Failed to send to unbounded channel");
        }
        rt.poll_until_idle();

        if let Some(next_time) = rt.next_time_poll() {
            self.schedule_at(Message::new().kind(RT_TIME_WAKEUP).build(), next_time);
        }

        // (1) Suspend the time context
        self.async_ext.time_context = Some(guard.leave());
    }

    fn activity(&mut self) {
        let rt = Arc::clone(&self.globals().runtime);
        let guard = rt.enter_time_context(self.async_ext.time_context.take().unwrap());

        rt.poll_time_events();
        {
            let self_ptr: *mut T = self;
            let self_ref: &'static mut T = unsafe { &mut *self_ptr };

            let join = rt.spawn(<T as AsyncModule>::activity(self_ref));
            let _result = rt.block_or_idle_on(join);
        }
        rt.poll_until_idle();

        if let Some(next_time) = rt.next_time_poll() {
            self.schedule_at(Message::new().kind(RT_TIME_WAKEUP).build(), next_time);
        }

        self.async_ext.time_context = Some(guard.leave());
    }

    fn at_sim_start(&mut self, stage: usize) {
        // time is 0

        let rt = Arc::clone(&self.globals().runtime);
        let guard = rt.enter_time_context(self.async_ext.time_context.take().unwrap());
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
            self.schedule_at(Message::new().kind(RT_TIME_WAKEUP).build(), next_time);
        }

        self.async_ext.time_context = Some(guard.leave());
    }

    fn finish_sim_start(&mut self) {
        let rt = Arc::clone(&self.globals().runtime);
        let guard = rt.enter_time_context(self.async_ext.time_context.take().unwrap());

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
            self.schedule_at(Message::new().kind(RT_TIME_WAKEUP).build(), next_time);
        }

        self.async_ext.time_context = Some(guard.leave());
    }

    fn at_sim_end(&mut self) {
        let rt = Arc::clone(&self.globals().runtime);
        let guard = rt.enter_time_context(self.async_ext.time_context.take().unwrap());
        rt.poll_time_events();
        {
            // SAFTEY:
            // Sim end means only this function will be executed before drop
            // thus 'static can be assumed.
            let self_ptr: *mut T = self;
            let self_ref: &'static mut T = unsafe { &mut *self_ptr };

            self.async_ext.sim_end_join = Some(rt.spawn(<T as AsyncModule>::at_sim_end(self_ref)));
        }
        rt.poll_until_idle();

        // No time event enqueue needed, wont be resolved either way

        self.async_ext.time_context = Some(guard.leave());
    }

    fn finish_sim_end(&mut self) {
        let rt = Arc::clone(&self.globals().runtime);
        let guard = rt.enter_time_context(self.async_ext.time_context.take().unwrap());
        rt.poll_time_events();
        rt.poll_until_idle();

        rt.block_or_idle_on(self.async_ext.sim_end_join.take().expect("Theif"))
            .expect("Join Idle")
            .expect("Join Error");

        // No time event enqueue needed, wont be resolved either way

        self.async_ext.time_context = Some(guard.leave());
    }

    fn num_sim_start_stages(&self) -> usize {
        // Needs at least one sim_start stage to setup the recv handle
        <T as AsyncModule>::num_sim_start_stages(self).min(1)
    }
}
