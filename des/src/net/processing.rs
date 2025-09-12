//! Module-specific plugins.
//!
//! Plugins act as message stream manipulators between the
//! main application and the network layer. They can be used
//! to add shared behaviour (like Routing) to all modules,
//! independent of the modules defined state and behaviour.
//!
//! All plugins must implement the `Plugin` trait. To install
//! them on a module, use the `add_plugin`
//! function and assign them a priority. The
//! lower the priority value, the closer the plugin is to the network
//! layer. Plugins can then be controlled and observed using the
//! `PluginHandle` return by the install functions.
//!
//! # Stream manipulation & event lifecycle
//!
//! Plugins are intrinsicly linked to the event lifecycle of an
//! arriving message. Accordingly they provide an API do react
//! to lifecycle events like `Plugin::event_start` and `Plugin::event_end`.
//!
//! When a message arrives at a module, the `Plugin::event_start` method
//! is called on all active plugins, in the order defined by the priorities
//! (close to networklayer first). Then the incoming message is passed
//! through the plugins in the same order. Plugins can capture messages
//! using the `Plugin::capture_incoming` method. Using this method
//! plugins can **modify**, **delete** or **pass through** messages.
//! Should they delete a message, no further plugins will be called
//! using `Plugin::capture_incoming`. Additionally no message will be
//! passed to the main application (defined by the module).
//!
//! If the message still exist after passing all plugins, then
//! it will be passed to the main application though `Module::handle_message`
//!
//! In the process of handeling an incoming message, each plugin and the main application
//! may send new messages to the networklayer using `send`
//! or `schedule_in`. This messages must pass through
//! all plugins (in reverse priority order). In this process they can be
//! captured and thus **modified** or **deleted** by all plugins, closer
//! to the network layer, than the message origin. This is done
//! using the `Plugin::capture_outgoing` method. If messages
//! make it through all plugins they will be added to the networklayer,
//! if not then not.
//!
//! After the main application has finished the message processing
//! the plugins are going to be deactivated in reverse order.
//! by calling `Plugin::event_end`. Sending messages at this stage will
//! still create new output-streams through all plugins closer to the networklayer
//! than the origin.
//!
//! # Plugin creation and removal
//!
//! When plugins are created using e.g. `add_plugin` they are not active
//! right away. Plugins only become active when the next event arrives.
//! This is the case, because some plugins may depend on some action
//! they should have performed in the incoming stream, when working on the
//! outgoing stream. However plugins may be created in a position, where their place
//! in the incoming stream should have allready been processed, but was not,
//! since they were not existent back then. Accordingly plugins only become active once they
//! can ensure that they existed at all relevent points in the event-lifecycle.
//!
//! Accordingly plugins the are removed using `PluginHandle::remove`
//! still exists for the rest of the event cycle, and are only deleted
//! once the next event arrives.
//!

use std::{any::Any, fmt::Debug, ops::Deref};

use crate::{net::module::Module, prelude::Message};

/// A subprogramm between the module application and the network layer.
///
/// Processing elements can follow different patterns based on the provided
/// API. Common patterns are:
///
/// - **Observer**: The element does not modifiy the message stream, it just observes it.
///   This plugin can be used to get statistics over message streams or to log
///   debug output.
/// - **Scope-Provider**: This element provides some kind of scope to all items further
///   from the network layer than itself. A scope can be defined using a static variable
///   or just consist of a time meassurement between start & end of the inner computation.
/// - **Capture**: This kind of processing element captures parts of the input stream and redirects
///   it in some abitraty way, using other APIs. This pattern can be used to implement buffering
///   or mergeing of frameneted IP packets.
/// - **Meta-Provider**: This kind of processing element attaches / modifies part of the incoming or
///   outgoing message stream to provide some new level of abstraction e.g. a VPN
///   or simulated network Interfaces.
pub trait ProcessingElement: Any {
    /// A simplifed capture clause that can modify an incoming message.
    ///
    /// This function is called at most once per event,
    /// but before all the main application has processed its message.
    ///
    /// This function receives an incoming message, and can
    /// modify, pass-through or delete a message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// struct Filter {
    ///    filter: Box<dyn Fn(&Message) -> bool>,
    /// }
    ///
    /// impl ProcessingElement for Filter {
    ///     fn process(&mut self, msg: Message) -> Option<Message> {
    ///        let f = &self.filter;
    ///        if f(&msg) {
    ///            Some(msg)
    ///        } else {
    ///            None
    ///        }
    ///     }
    /// }
    /// ```
    fn process(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }

    /// A general capture clause that reacts to abitrary node activations and encapsulates
    /// further processing elements. Use this function to perform work on more than just
    /// `HandleMessage` events and to set scope variables.
    ///
    /// This function is called for a variety of node activations:
    /// - an arriving message
    /// - a sim-start or sim-end stage
    /// - an async wakeup event
    ///
    /// An event is than passed through a chain of processing elements. The `i-th` processing elements
    /// will receive the output from the `(i-1)-th` processing element and a closure that will execute the
    /// `(i+1)-th` processing element. The last processing element will be the module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// struct MeassureTimer {
    ///     meassurements: Vec<f64>,
    /// }
    ///
    /// impl ProcessingElement for MeassureTimer {
    ///     fn process_with(
    ///         &mut self,
    ///         msg: Option<Message>,
    ///         inner: &mut dyn FnMut(Option<Message>) -> Option<Message>,
    ///     ) -> Option<Message> {
    ///         let t0 = std::time::Instant::now();
    ///         let res = inner(msg);
    ///         self.meassurements.push(t0.elapsed().as_millis() as f64);
    ///         res
    ///     }
    /// }
    ///
    /// ```
    fn process_with(
        &mut self,
        msg: Option<Message>,
        inner: &mut dyn FnMut(Option<Message>) -> Option<Message>,
    ) -> Option<Message> {
        let res = msg.and_then(|msg| self.process(msg));
        inner(res)
    }
}

/// A untyped set of processing elements, effectivly a processing stack.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ModuleImpl {
    pub(super) stack: ProcessingStack,
    pub(super) handler: Box<dyn Module>,
}

impl ModuleImpl {
    pub(super) fn new(stack: ProcessingStack, handler: Box<dyn Module>) -> Self {
        ModuleImpl { stack, handler }
    }

    // FIXME: O(n) lookups
    // This lookup operations scales O(n) with the amount of proc-elements
    // maybe make a lookup using a BTreeMap?

    pub(super) fn downcast_element_ref<T: Any>(&self) -> Option<&T> {
        for element in &self.stack.items {
            let as_any: &dyn Any = &**element;
            if let Some(element) = as_any.downcast_ref::<T>() {
                return Some(element);
            }
        }

        let as_any: &dyn Any = &*self.handler;
        as_any.downcast_ref::<T>()
    }

    pub(super) fn downcast_element_mut<T: Any>(&mut self) -> Option<&mut T> {
        for element in &mut self.stack.items {
            let as_any: &mut dyn Any = &mut **element;
            if let Some(element) = as_any.downcast_mut::<T>() {
                return Some(element);
            }
        }

        let as_any: &mut dyn Any = &mut *self.handler;
        as_any.downcast_mut::<T>()
    }

    // NOTE:
    // it is fundamentally impossible to access proc-elements from within the active module,
    // since by the design of process_with the element is already mutable borrowed. While we could argue
    // that the borrow is lifted for the duration of the inner call, modelling this is rather complicated
    // so better not do it.

    pub(super) fn process_with<R>(
        &mut self,
        msg: Option<Message>,
        mut inner: impl FnMut(&mut dyn Module, Option<Message>) -> R,
    ) -> R {
        let mut slot = None;
        chain_processing_elements(&mut self.stack.items[..], msg, &mut |msg| {
            slot = Some(inner(&mut *self.handler, msg));
        });
        slot.take().expect("failed to execute inner closure")
    }
}

// This recursive function may be horribly inefficient
// TODO: check asm output / actual performance
fn chain_processing_elements<R>(
    elements: &mut [Box<dyn ProcessingElement>],
    msg: Option<Message>,
    inner: &mut impl FnMut(Option<Message>) -> R,
) -> Option<Message> {
    if elements.is_empty() {
        (inner)(msg);
        None
    } else {
        let (first, rest) = elements.split_at_mut(1);
        first[0].process_with(msg, &mut |msg| chain_processing_elements(rest, msg, inner))
    }
}

/// A stack of processing elements
pub struct ProcessingStack {
    items: Vec<Box<dyn ProcessingElement>>,
}

impl ProcessingStack {
    /// Merge a new stack onto the the current one.
    pub fn append(&mut self, expansion: impl Into<ProcessingStack>) {
        self.items.extend(expansion.into().items);
    }
}

impl Debug for ProcessingStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessingStack").finish()
    }
}

impl Deref for ProcessingStack {
    type Target = [Box<dyn ProcessingElement>];
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl From<()> for ProcessingStack {
    fn from((): ()) -> Self {
        ProcessingStack::default()
    }
}

impl Default for ProcessingStack {
    fn default() -> Self {
        #[cfg(feature = "async")]
        return ProcessingStack {
            items: vec![
                Box::new(TimeDriver::default()),
                Box::new(TokioRuntime::default()),
            ],
        };

        #[cfg(not(feature = "async"))]
        return ProcessingStack { items: Vec::new() };
    }
}

impl<P: ProcessingElement> From<P> for ProcessingStack {
    fn from(value: P) -> Self {
        ProcessingStack {
            items: vec![Box::new(value)],
        }
    }
}

macro_rules! for_tuples {
    (
        $($i:ident),*
    ) => {
        impl<$($i: ProcessingElement + 'static),*> From<($($i),*)> for ProcessingStack {
            #[allow(non_snake_case)]
            fn from(value: ($($i),*)) -> Self {
                let mut stack = ProcessingStack { items: Vec::new()};
                let ($($i),*) = value;
                $(
                    stack.append(ProcessingStack::from($i));
                )*
                stack
            }
        }
    };
}

for_tuples!(A, B);
for_tuples!(A, B, C);
for_tuples!(A, B, C, D);
for_tuples!(A, B, C, D, E);
for_tuples!(A, B, C, D, E, F);
for_tuples!(A, B, C, D, E, F, G);
for_tuples!(A, B, C, D, E, F, G, H);
for_tuples!(A, B, C, D, E, F, G, H, I);
for_tuples!(A, B, C, D, E, F, G, H, I, J);

cfg_async! {
    use std::{
        cell::LazyCell,
        iter::once,
        rc::Rc,
        sync::{Arc, LazyLock, Mutex},
    };

    use tokio::{
        runtime::{Builder, RngSeed, Runtime},
        task::{JoinHandle, LocalSet, yield_now},
    };

    use crate::{
        net::{
            Error, ErrorKind, JoinErrorKind,
            runtime::{NetEvents, AsyncWakeupEvent},
            schedule_event,
        },
        prelude::{RuntimeError, current, random},
        time::Driver,
    };

    /// A processing element that provides a tokio runtime in the entered state.
    #[derive(Debug)]
    pub struct TokioRuntime {
        pub(super) tasks: Rc<LocalSet>,
        pub(super) rt: LazyCell<Arc<Runtime>>,
        pub(super) handles: Vec<(JoinHandle<()>, bool)>,
    }

    #[allow(clippy::type_complexity)]
    static JOIN_THREADS: LazyLock<Mutex<Vec<(JoinHandle<()>, bool)>>> =
        LazyLock::new(Mutex::default);

    impl Default for TokioRuntime {
     fn default() -> Self {
            let tasks = Rc::new(LocalSet::new());
            Self {
                tasks,
                rt: LazyCell::new(|| {
                    #[allow(unused_mut)]
                    let mut builder = Builder::new_current_thread();
                    #[cfg(feature = "unstable-tokio-enable-time")]
                    builder.enable_time();

                    Arc::new(
                        builder
                            .rng_seed(RngSeed::from_bytes(&random::<u64>().to_le_bytes()))
                            .build()
                            .expect("Failed to build tokio runtime"),
                    )
                }),
                handles: Vec::new(),
            }
        }
    }

    impl TokioRuntime {
        /// Join a handle.
        #[allow(clippy::missing_panics_doc)]
        pub fn join(handle: JoinHandle<()>) {
            let mut join_threads = JOIN_THREADS.lock().expect("failed to get lock");
            join_threads.push((handle, true));
        }

        /// Try to join a handle.
        #[allow(clippy::missing_panics_doc)]
        pub fn try_join(handle: JoinHandle<()>) {
            let mut join_threads = JOIN_THREADS.lock().expect("failed to get lock");
            join_threads.push((handle, false));
        }

        /// Reset the join handles.
        #[allow(clippy::missing_panics_doc)]
        pub fn reset_join_handles(&mut self) {
            let mut join_threads = JOIN_THREADS.lock().expect("failed to get lock");
            join_threads.clear();
            self.handles.clear();
        }

        /// Reset the runtime.
        pub fn reset(&mut self) {
            *self = Self::default();
        }

        /// Shutdown the runtime.
        pub fn shutdown(&mut self) {
            *self = Self::default();
        }

        /// a custom handler for sim-end szenarios, only supported by this proc-element.
        ///
        /// # Errors
        ///
        /// Erorors that occured in handles about to be joined.
        pub fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
            let mut error = RuntimeError::empty();

            let _guard = self.rt.enter();
            for (handle, must_join) in self.handles.drain(..) {
                if !handle.is_finished() {
                    if must_join {
                        error.extend(once(Error::new_current(ErrorKind::JoinError(
                            JoinErrorKind::NotFinished,
                        ))));
                    }
                    continue;
                }

                match self.rt.block_on(handle) {
                    Ok(()) => {}
                    Err(e) if e.is_panic() => error.extend(once(Error::new_current(
                        ErrorKind::JoinError(JoinErrorKind::Paniced(e.into_panic())),
                    ))),
                    Err(e) => error.extend(once(Error::new_current(ErrorKind::JoinError(
                        JoinErrorKind::Tokio(e),
                    )))),
                }
            }

            if error.is_empty() { Ok(()) } else { Err(error) }
        }
    }

    impl ProcessingElement for TokioRuntime {
        fn process_with(
            &mut self,
            msg: Option<Message>,
            inner: &mut dyn FnMut(Option<Message>) -> Option<Message>,
        ) -> Option<Message> {
            JOIN_THREADS.lock().expect("failed to get lock").clear();

            let res = self.tasks.block_on(&self.rt, async {
                let res = inner(msg);
                yield_now().await;
                res
            });

            self.handles.append(
                &mut JOIN_THREADS
                    .lock()
                    .expect("failed to get lock, this should be impossible"),
            );

            res
        }
    }

    /// Timer Driver
    #[derive(Debug)]
    pub struct TimeDriver {
        driver: Option<Driver>,
    }

    impl Default for TimeDriver {
        fn default() -> Self {
            Self {
                driver: Some(Driver::new()),
            }
        }
    }

    impl ProcessingElement for TimeDriver {
        fn process_with(
            &mut self,
            msg: Option<Message>,
            inner: &mut dyn FnMut(Option<Message>) -> Option<Message>,
        ) -> Option<Message> {
            use crate::time::{SimTime, TimerSlot};

            let driver = self.driver.take();
            if let Some(mut driver) = driver {
                let bumpable = driver.bump();
                if driver.next_wakeup <= SimTime::now() {
                    driver.next_wakeup = SimTime::MAX;
                }
                bumpable.into_iter().for_each(TimerSlot::wake_all);
                driver.set();
            }

            let res = inner(msg);

            let Some(mut driver) = Driver::unset() else {
                // Somebody stole our driver
                #[cfg(feature = "tracing")]
                tracing::error!("IO time driver missing after event execution");

                self.driver = Some(Driver::new());
                return res;
            };

            if let Some(next_wakeup) = driver.next()
                && next_wakeup < driver.next_wakeup
            {
                #[cfg(feature = "tracing")]
                tracing::trace!(
                    "scheduling new wakeup at {} (prev {})",
                    next_wakeup,
                    driver.next_wakeup
                );

                driver.next_wakeup = next_wakeup;

                schedule_event(
                    NetEvents::AsyncWakeupEvent(AsyncWakeupEvent {
                        module: current().me(),
                    }),
                    next_wakeup,
                );
            }

            self.driver = Some(driver);
            res
        }
    }
}
