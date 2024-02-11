#![allow(clippy::unused_async)]

use crate::net::{message::Message, module::Module};
use crate::time::SimTime;
use tokio::task::yield_now;

pub(crate) mod core;
pub(crate) use self::core::WaitingMessage;

/// A set of user defined functions for customizing the behaviour
/// of an asynchronous module.
///
/// This trait is just a async version of [`Module`](crate::net::module::Module).
#[allow(async_fn_in_trait)]
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
    /// event execution use [`tokio::spawn`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::prelude::*;
    ///
    /// struct MyAsyncModule {
    ///     prop_1: f64,
    ///     prop_2: String,
    /// }
    ///
    /// 
    /// impl AsyncModule for MyAsyncModule {
    /// # fn new() -> Self { todo!() }
    ///     /* ... */    
    ///
    ///     async fn handle_message(&mut self, msg: Message) {
    ///         println!("Received {:?}", msg);
    ///     }
    /// }
    /// ```
    async fn handle_message(&mut self, _: Message) {}

    ///
    /// A function that is run at the start of each simulation, for each module.
    /// The order in which modules are called is not guranteed but the stage numbers are.
    /// That means that all stage-0 calls for all modules happen before the first (if any) stage-1 calls.
    /// Generaly speaking, all stage-i calls finish before the first stage-i+1 call.
    ///
    /// # Note
    ///
    /// The user must ensure that all calls of `at_sim_start` will terminate at last
    /// once all stages of `at_sim_start` of all modules have been called.
    /// The stages will be executed in order.
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::prelude::*;
    ///
    /// # type Config = ();
    /// async fn fetch_config(id: ModuleId) -> Config {
    ///     // ...
    /// }
    ///
    /// struct MyModule {
    ///     config: Config,
    ///     records: Vec<f64>,
    /// }
    ///
    /// 
    /// impl AsyncModule for MyModule {
    /// # fn new() -> Self { todo!() }
    ///     /* ... */    
    ///
    ///     async fn handle_message(&mut self, _: Message) {
    ///         // ...
    ///     }
    ///
    ///     async fn at_sim_start(&mut self, _stage: usize) {
    ///         self.config = fetch_config(current().id()).await;
    ///         self.records.clear();
    ///     }
    /// }
    ///
    /// ```
    async fn at_sim_start(&mut self, _: usize) {}

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
    fn __indicate_async(&self) -> bool {
        true
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        <T as AsyncModule>::new()
    }

    fn reset(&mut self) {
        super::async_ctx_reset();

        <T as AsyncModule>::reset(self);
    }

    fn handle_message(&mut self, msg: Message) {
        // (1) Fetch the runtime and initial the time context.
        let Some(rt) = super::async_get_rt() else {
            return;
        };

        // (2) Ignore notifty message only relevant for a
        // call to poll_until_idle
        super::async_wait_queue_tx_send(WaitingMessage {
            msg,
            time: SimTime::now(),
        })
        .expect("Failed to forward message to 'handle_message'");

        rt.1.block_on(&rt.0, yield_now());
    }

    fn at_sim_start(&mut self, stage: usize) {
        // time is 0
        let Some(rt) = super::async_get_rt() else {
            return;
        };

        // # Setup message receive handle.
        if stage == 0 {
            // SAFTEY:
            // We can guarantee the validity of the pointer:
            // 1) The module is pinned while the simulation is running.
            // 2) The module is not dropped while the simulation is running.
            // 3) While we may create mutiple &mut T, handle_message is never run fully
            //    async (current thread runtime) and mutiple calls of `handle_messsage`
            //    wont overlap, since the queue rx synchronises and delays them.
            // 4) References to at_sim_start have been droped since all futures of at_sim_start
            //    must be resoved before event 1
            let self_ref: &'static mut T = {
                let ptr: *mut T = self;
                unsafe { &mut *ptr }
            };

            let mut rx = super::async_wait_queue_rx_take().expect("We have been robbed");

            super::async_set_wait_queue_join(rt.1.spawn_local(async move {
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

            super::async_set_sim_start_join(rt.1.spawn_local(async move {
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

        rt.1.block_on(&rt.0, yield_now());
    }

    fn finish_sim_start(&mut self) {
        let Some(rt) = super::async_get_rt() else {
            return;
        };

        super::async_sim_start_tx_send(usize::MAX)
            .expect("Failed to send close signal to sim_start_task");

        rt.1.block_on(&rt.0, yield_now());

        // The join must succeed else saftey invariant cannot be archived.
        let handle = super::async_sim_start_join_take().expect("Crime");
        let _g = rt.0.enter();
        assert!(handle.is_finished());
        rt.0.block_on(handle).unwrap();
    }

    fn at_sim_end(&mut self) {
        let Some(rt) = super::async_get_rt() else {
            return;
        };

        // SAFTEY:
        // Sim end means only this function will be executed before drop
        // thus 'static can be assumed.
        let self_ptr: *mut T = self;
        let self_ref: &'static mut T = unsafe { &mut *self_ptr };

        super::async_sim_end_join_set(rt.1.spawn_local(<T as AsyncModule>::at_sim_end(self_ref)));

        rt.1.block_on(&rt.0, yield_now());
    }

    fn finish_sim_end(&mut self) {
        let Some(rt) = super::async_get_rt() else {
            return;
        };

        rt.1.block_on(&rt.0, yield_now());

        let handle = super::async_sim_end_join_take().expect("Crime");
        let _g = rt.0.enter();
        assert!(
            handle.is_finished(),
            "at_sim_end() could not complete, since it is stuck at some await point"
        );
        rt.0.block_on(handle).unwrap();
    }

    fn num_sim_start_stages(&self) -> usize {
        // Needs at least one sim_start stage to setup the recv handle
        <T as AsyncModule>::num_sim_start_stages(self).max(1)
    }
}
