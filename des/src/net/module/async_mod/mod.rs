mod handle;
pub use handle::*;

use crate::time::SimTime;
use crate::net::{Message, MessageKind, Module, StaticModuleCore};
use async_trait::async_trait;

pub const RT_TIME_WAKEUP: MessageKind = 42;

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
    fn num_sim_start_stages(&self) -> usize { 1 }
}

impl<T> Module for T where T: AsyncModule   {
    fn handle_message(&mut self, msg: Message) {
        tokio::time::SimTime::set_now(SimTime::now().into());

        let rt = self.module_core().runtime.clone();
        rt.poll_time_events();
        if msg.meta().kind != RT_TIME_WAKEUP {
            let _result = rt.block_or_idle_on(<T as AsyncModule>::handle_message(self, msg));
        }
        rt.poll_until_idle();

        if let Some(next_time) = rt.next_time_poll() {
            self.schedule_at(Message::new().kind(RT_TIME_WAKEUP).build(), next_time.into())
        }
    }

    fn activity(&mut self) {
        tokio::time::SimTime::set_now(SimTime::now().into());

        let rt = self.module_core().runtime.clone();
        rt.poll_time_events();
        let _result = rt.block_or_idle_on(<T as AsyncModule>::activity(self));
        rt.poll_until_idle();

        if let Some(next_time) = rt.next_time_poll() {
            self.schedule_at(Message::new().kind(RT_TIME_WAKEUP).build(), next_time.into())
        }
    }

    fn at_sim_start(&mut self, stage: usize) {
        // time is 0

        let rt = self.module_core().runtime.clone();
        rt.poll_time_events();
        let _result = rt.block_or_idle_on(<T as AsyncModule>::at_sim_start(self, stage));
        rt.poll_until_idle();

        if let Some(next_time) = rt.next_time_poll() {
            self.schedule_at(Message::new().kind(RT_TIME_WAKEUP).build(), next_time.into())
        }
    }

    fn at_sim_end(&mut self) {
        tokio::time::SimTime::set_now(SimTime::now().into());

        let rt = self.module_core().runtime.clone();
        rt.poll_time_events();
        let _result = rt.block_or_idle_on(<T as AsyncModule>::at_sim_end(self));
        rt.poll_until_idle();

        if let Some(next_time) = rt.next_time_poll() {
            self.schedule_at(Message::new().kind(RT_TIME_WAKEUP).build(), next_time.into())
        }
    }

    fn num_sim_start_stages(&self) -> usize {
        <T as AsyncModule>::num_sim_start_stages(self)
    }
}