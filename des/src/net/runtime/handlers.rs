//! Custom module blocks that simplify the `Module` API.

use crate::{
    net::{message::Message, module::Module},
    prelude::current,
};
use std::{error::Error, time::Duration};

/// The policy that descibes how a module should proceeed, if a
/// handler function returns an error.
#[derive(Debug, Clone, Copy)]
pub enum FailabilityPolicy {
    /// This option causes the module to panic with the error.
    ///
    /// Only use this option if errors indicate that something went so wrong,
    /// the entire simulation should fail.
    Panic,
    /// This option causes the module to just continue as is. The error will be logged.
    ///
    /// Only use this option if you can ensure that the error has not caused an invalid
    /// state, should the node be statefull.
    Continue,
    /// This option triggers a node restart as a result of an error.
    ///
    /// If set on a node without restart semantics this is equivalent to `Continue`
    Restart,
}

/// A wrapper for treating a handler functions as a module.
///
/// This wrapper takes an `FnMut(Message) -> ?` as input at uses this function
/// as message handler, called by [`Module::handle_message`]. Use this wrapper
/// if a node software is stateless, an can be simply described by a handler function
/// alone.
///
/// Since this wrapper is stateless, restarting it will have no effect on the
/// internals.
///
/// # Examples
///
/// ```
/// # use des::prelude::*;
/// # use des::net::handlers::HandlerFn;
/// let mut sim = Sim::new(());
/// sim.node("alice", HandlerFn::new(|msg| {
///     /* Do something stateless (e.g. random routing) */
/// }));
///
/// let _ = Builder::new().build(sim.freeze()).run();
/// ```
#[derive(Debug)]
pub struct HandlerFn<Handler> {
    inner: Handler,
}

impl<Handler> HandlerFn<Handler>
where
    Handler: FnMut(Message),
{
    /// Creates a new wrapper for a function returning the unit type.
    pub fn new(handler: Handler) -> Self {
        Self { inner: handler }
    }
}

impl<Handler, Err> HandlerFn<Handler>
where
    Handler: FnMut(Message) -> Result<(), Err>,
    Err: Error,
{
    /// Creates a new wrapper for a function returning some `Result`.
    ///
    /// The parameter `policy` defines how an error will be processed, should
    /// one occur during the execution of the handler.
    #[allow(clippy::missing_panics_doc)]
    pub fn failable(
        mut handler: Handler,
        policy: FailabilityPolicy,
    ) -> HandlerFn<impl FnMut(Message)> {
        HandlerFn {
            inner: move |msg| match handler(msg) {
                Ok(()) => {}
                Err(e) => match policy {
                    FailabilityPolicy::Panic => panic!(
                        "node '{}' failed to process message, handler fn failed with: {e} ",
                        current().path
                    ),
                    FailabilityPolicy::Continue | FailabilityPolicy::Restart => {
                        tracing::error!("failed to process message, handler fn failed with: {e}");
                    }
                },
            },
        }
    }
}

impl<Handler> Module for HandlerFn<Handler>
where
    Handler: FnMut(Message) + 'static,
{
    fn handle_message(&mut self, msg: Message) {
        (self.inner)(msg);
    }
}

/// A wrapper for creating handler functions with state, treatable as a module.
///
/// This wrapper takes two functions, one to create some state `FnMut() -> State` and
/// a handler function that accepts the state as an additional parameter `FnMut(&mut State, Message)`
/// to create a module. The generator function will be executed once the simulation has been started,
/// within module-scope. When a module is shut down, the generator function can be used to reinitalize the
/// state. Note that `gen` can be used for other things than just initalizing the state.
///
/// # Examples
///
/// ```
/// # use des::prelude::*;
/// # use des::net::handlers::ModuleFn;
/// struct State {
///     /* ...data */
/// }
///
/// let mut sim = Sim::new(());
/// sim.node("alice", ModuleFn::new(
///     || {
///         /* Treat this as at_sim_start() */
///         State { /* ...data */ }
///     },
///     |state, msg| {
///         /* Do some message processing here */
///     }
/// ));
///
/// let _ = Builder::new().build(sim.freeze()).run();
/// ```
#[derive(Debug)]
pub struct ModuleFn<Gen, State, Handler> {
    generator: Gen,
    current: Option<State>,
    handler: Handler,
}

impl<Gen, State, Handler> ModuleFn<Gen, State, Handler>
where
    Gen: FnMut() -> State,
    Handler: FnMut(&mut State, Message),
{
    /// Creates a wrapper over a function that returns the unit type.
    pub fn new(generator: Gen, handler: Handler) -> Self {
        Self {
            handler,
            current: None,
            generator,
        }
    }
}

impl<Gen, State, Handler, Err> ModuleFn<Gen, State, Handler>
where
    Gen: FnMut() -> State,
    Handler: FnMut(&mut State, Message) -> Result<(), Err> + 'static,
    Err: Error,
{
    /// Creates a new wrapper for a function returning some `Result`.
    ///
    /// The parameter `policy` defines how an error will be processed, should
    /// one occur during the execution of the handler.
    #[allow(clippy::missing_panics_doc)]
    #[allow(clippy::complexity)]
    pub fn failable(
        generator: Gen,
        mut handler: Handler,
        policy: FailabilityPolicy,
    ) -> ModuleFn<Gen, State, Box<dyn FnMut(&mut State, Message)>> {
        ModuleFn {
            current: None,
            generator,
            handler: Box::new(move |state, msg| match handler(state, msg) {
                Ok(()) => {}
                Err(e) => match policy {
                    FailabilityPolicy::Panic => panic!(
                        "node '{}' failed to process message, handler fn failed with: {e} ",
                        current().path
                    ),
                    FailabilityPolicy::Continue => {
                        tracing::error!("failed to process message, handler fn failed with: {e}");
                    }
                    FailabilityPolicy::Restart => {
                        tracing::error!("failed to process message, handler fn failed with: {e}");
                        current().shutdow_and_restart_in(Duration::ZERO);
                    }
                },
            }),
        }
    }
}

impl<Gen, State, Handler> Module for ModuleFn<Gen, State, Handler>
where
    Gen: FnMut() -> State + 'static,
    State: 'static,
    Handler: FnMut(&mut State, Message) + 'static,
{
    fn reset(&mut self) {
        self.current = None;
    }

    fn at_sim_start(&mut self, _stage: usize) {
        self.current = Some((self.generator)());
    }

    fn handle_message(&mut self, msg: Message) {
        let Some(state) = &mut self.current else {
            unreachable!("handle_message cannot be called before at_sim_start")
        };
        (self.handler)(state, msg);
    }
}

cfg_async! {
    use tokio::{
        sync::mpsc::{self, Receiver, Sender},
    };
    use std::{future::Future, fmt::Formatter, pin::Pin};
    use crate::{runtime::RuntimeError};


    /// A helper that enables user to treat a module as a async stream of messages,
    /// with state attached.
    ///
    /// This helper enables user to use `async FnMut(Receiver<Message>)` as a
    /// module. The provided function is called at the start of the simulation, within
    /// module context. The provided receiver will stream incoming message to the
    /// async clousure. Any user code should saturate the mpsc channel asap, since
    /// it is bounded. Message buffering should be implemented manually.
    ///
    /// On module restarts, the generator function will be called again with a new
    /// receiver.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::net::handlers::AsyncHandler;
    /// let mut sim = Sim::new(());
    /// sim.node("alice", AsyncHandler::new(|mut rx| {
    ///     /* Do some setup / sim_start_stuff here */
    ///     async move {
    ///         while let Some(msg) = rx.recv().await {
    ///             /* Message processing */
    ///         }
    ///     }
    /// }));
    /// /* ... */
    ///
    /// let _ = Builder::new().build(sim.freeze()).run();
    /// ```
    pub struct AsyncHandler
    {
        generator: BoxedGen,

        tx: Sender<Message>,
        rx: Option<Receiver<Message>>,

        require_recv: bool,
        require_join: bool,
    }

    type BoxedGen = Box<dyn FnMut(Receiver<Message>) -> BoxedFuture + Send>;
    type BoxedFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

    impl AsyncHandler {
        /// Sets the handler to require a join
        #[must_use]
        pub fn require_join(mut self) -> Self {
            self.require_join = true;
            self
        }

        /// Configures the handler so, that all incoming packets must be
        /// read from the `rx` queue. If that does not happen, the module will panic.
        #[must_use]
        pub fn require_recv(mut self) -> Self {
            self.require_recv = true;
            self
        }

        /// Creates a new instance using the generator function.
        pub fn new<Gen, Fut>(mut generator: Gen) -> Self
        where
            Gen: FnMut(Receiver<Message>) -> Fut,
            Gen: Send + 'static,
            Fut: Future<Output = ()>,
            Fut: Send + 'static,
        {
            let (tx, rx) = mpsc::channel(8);
            Self {
                generator: Box::new(move |rx| Box::pin(generator(rx))),
                tx,
                rx: Some(rx),
                require_join: false,
                require_recv: false
            }
        }

        /// Creates a new instance using the generator function.
        #[allow(clippy::missing_panics_doc)]
        pub fn failable<Failable, Fut, Err>(mut generator: Failable) -> Self
        where
            Failable: FnMut(Receiver<Message>) -> Fut,
            Failable: Send + 'static,
            Fut: Future<Output = Result<(), Err>>,
            Fut: Send + 'static,
            Err: Error
        {
            let (tx, rx) = mpsc::channel(8);
            Self {
                generator: Box::new(move |rx| {
                    let fut = generator(rx);
                    Box::pin(async move {
                        match fut.await {
                            Ok(()) => {},
                            Err(e) => {
                                panic!("node {} paniced at failable operation: {e}", current().path());
                            },
                        }
                    })
                }),
                tx,
                rx: Some(rx),
                require_join: false, // TODO: make error without join
                require_recv: false,
            }
        }

        /// Makes an `std::io::error` exepector
        pub fn io<Gen, Fut>(generator: Gen) -> Self
        where
            Gen: FnMut(Receiver<Message>) -> Fut,
            Gen: Send + 'static,
            Fut: Future<Output = std::io::Result<()>>,
            Fut: Send + 'static,
        {
            Self::failable(generator)
        }
    }

    impl Module for AsyncHandler {
        fn reset(&mut self) {
            // FIXME: we should reset the join handles here, but this will be reworked next so ignore
        }

         fn at_sim_start(&mut self, _: usize) {
            let rx = self.rx.take().unwrap_or_else(|| {
                let (tx, rx) = mpsc::channel(8);
                self.tx = tx;
                rx
            });

            let fut = (self.generator)(rx);
            let fut = async move {
                fut.await;
            };

            if self.require_join {
                current().join(tokio::spawn(fut));
            } else {
                current().try_join(tokio::spawn(fut));
            }
        }

         fn handle_message(&mut self, msg: Message) {
            if let Err(e) = self.tx.try_send(msg) {
                assert!(!self.require_recv, "failed to receive an incoming packet: {e}");
            }
        }

         fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
            Ok(())
         }

    }

    impl std::fmt::Debug for AsyncHandler {
        fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
            write!(f, "AsyncFn")
        }
    }
}
