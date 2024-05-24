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

use std::{any::Any, sync::RwLock};

use super::module::Module;
use crate::prelude::Message;

/// A subprogramm between the module application and the network layer.
///
/// Processing elements can follow different patterns based on the provided
/// API. Common patterns are:
///
/// - **Observer**: The element does not modifiy the message stream, it just observes it.
///    This plugin can be used to get statistics over message streams or to log
///    debug output.
/// - **Scope-Provider**: This element provides some kind of scope to all items further
///    from the network layer than itself. A scope can be defined using a static variable
///    or just consist of a time meassurement between [`event_start`] / [`event_end`].
/// - **Capture**: This kind of processing element captures parts of the input stream and redirects
///     it in some abitraty way, using other APIs. This pattern can be used to implement buffering
///     or mergeing of frameneted IP packets.
/// - **Meta-Provider**: This kind of processing element attaches / modifies part of the incoming or
///    outgoing message stream to provide some new level of abstraction e.g. a VPN
///    or simulated network Interfaces.
///
/// [`event_start`]: ProcessingElement::event_start
/// [`event_end`]: ProcessingElement::event_end
pub trait ProcessingElement: Any {
    /// Defines the requires stack for this processing element.
    ///
    ///
    fn stack(&self) -> impl IntoProcessingElements
    where
        Self: Sized,
    {
        BaseLoader
    }

    /// A handler for when an the event processing of a message starts.
    ///
    /// This function is called only once per event. If this function is called
    /// this means all plugins closer to the network layer have allready been called
    /// while all plugins further from the network layer are not yet called.
    ///
    /// Use this function to set up actions, required at the start of
    /// a generic event.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use des::prelude::*;
    /// struct LoggerPlugin {
    ///     counter: usize,
    /// }
    ///
    /// impl ProcessingElement for LoggerPlugin {
    ///     fn event_start(&mut self) {
    ///         tracing::trace!("receiving {}th message", self.counter);
    ///         self.counter += 1;   
    ///     }
    /// }
    /// ```
    fn event_start(&mut self) {}

    /// A handler for when an the event processing of a message ends.
    ///
    /// This function is called only once per event. The call order
    /// is the reverse to the call order of [`event_start`].
    ///
    /// Use this function to set up actions, associated
    /// with the end of an event
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use des::prelude::*;
    /// # use des::time::*;
    /// struct Timer {
    ///     started: SimTime,    
    /// }
    ///
    /// impl ProcessingElement for Timer {
    ///     fn event_start(&mut self) {
    ///        self.started = SimTime::now();
    ///     }
    ///     fn event_end(&mut self) {
    ///        let t = SimTime::now().duration_since(self.started);
    ///        tracing::trace!("took {:?}", t);   
    ///     }
    /// }
    /// ```
    ///
    /// [`event_start`]: ProcessingElement::event_start
    fn event_end(&mut self) {}

    /// A capture clause that can modify an incoming message.
    ///
    /// This function is called at most once per event, after all
    /// plugins have called [`event_start`],
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
    ///     fn incoming(&mut self, msg: Message) -> Option<Message> {
    ///        let f = &self.filter;
    ///        if f(&msg) {
    ///            Some(msg)    
    ///        } else {
    ///            None
    ///        }
    ///     }    
    /// }
    /// ```
    ///
    /// [`event_start`]: ProcessingElement::event_start
    fn incoming(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }
}

/// A type that can be interprested as a processing element chain.
pub trait IntoProcessingElements: 'static {
    /// Convertes into processing elements
    fn to_processing_elements(self) -> Vec<ProcessorElement>;
}

impl<P: ProcessingElement + 'static> IntoProcessingElements for P {
    fn to_processing_elements(self) -> Vec<ProcessorElement> {
        let mut stack = self.stack().to_processing_elements();
        stack.push(ProcessorElement::new(self));
        stack
    }
}

impl<T: Module> ProcessingElement for T {
    fn stack(&self) -> impl IntoProcessingElements {
        <Self as Module>::stack(self)
    }

    fn incoming(&mut self, msg: Message) -> Option<Message> {
        self.handle_message(msg);
        None
    }
}

/// A base module that is used to load the default processing elements
/// onto a module.
///
/// It's common for simulations to share a set of basic processing elements
/// accross all nodes. The baseLoader is a maker type, that attaches all default
/// plugins to a node, that relies on `BaseLoader` in its processing stack.
///
/// Use [`set_default_processing_elements`] to set the default processing
/// elements.
#[derive(Debug)]
pub struct BaseLoader;

pub(crate) static SETUP_PROCESSING: RwLock<fn() -> Vec<ProcessorElement>> =
    RwLock::new(_default_processing);

fn _default_processing() -> Vec<ProcessorElement> {
    Vec::new()
}

impl IntoProcessingElements for BaseLoader {
    fn to_processing_elements(self) -> Vec<ProcessorElement> {
        SETUP_PROCESSING.try_read().expect("Cannot access fn")()
    }
}

/// Sets a handler to create the default processing element of a module
///
/// # Panics
///
/// May panic at interal misconfiguration
pub fn set_default_processing_elements(f: fn() -> Vec<ProcessorElement>) {
    *SETUP_PROCESSING.try_write().expect("no lock") = f;
}

/// A untyped set of processing elements, effectivly a processing stack.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ProcessingElements {
    // last element is module
    pub(super) state: ProcessingState,
    stack: Vec<ProcessorElement>,
    pub(super) handler: Box<dyn Module>,
}

/// A untyped processing element, using dynamic dispatch.
#[allow(missing_debug_implementations)]
pub struct ProcessorElement {
    inner: Box<dyn ProcessingElement>,
}

impl ProcessorElement {
    /// Creates a new type-erased wrapper around a concrete processing element.
    pub fn new<T: ProcessingElement>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

pub(super) enum ProcessingState {
    Upstream(usize), // next processing index
    Peek,
    Downstream(usize), // last processing index
}

impl ProcessingState {
    fn bump_upstream(&mut self) {
        match self {
            ProcessingState::Upstream(ref mut idx) => *idx += 1,
            _ => unreachable!(),
        }
    }

    fn bump_downstream(&mut self) {
        match self {
            ProcessingState::Downstream(ref mut idx) => *idx -= 1,
            _ => unreachable!(),
        }
    }
}

impl ProcessingElements {
    pub(super) fn new(stack: Vec<ProcessorElement>, handler: impl Module) -> Self {
        ProcessingElements {
            state: ProcessingState::Upstream(0),
            stack,
            handler: Box::new(handler),
        }
    }

    pub(super) fn incoming_upstream(&mut self, msg: Option<Message>) -> Option<Message> {
        self.state = ProcessingState::Upstream(0);

        let mut msg = msg;
        for i in 0..self.stack.len() {
            self.stack[i].inner.event_start();
            if let Some(existing_msg) = msg {
                msg = self.stack[i].inner.incoming(existing_msg);
            }
            self.state.bump_upstream();
        }
        msg
    }

    pub(super) fn incoming_downstream(&mut self) {
        self.state = ProcessingState::Downstream(self.stack.len());
        for i in (0..self.stack.len()).rev() {
            self.stack[i].inner.event_end();
            self.state.bump_downstream();
        }
    }
}

impl IntoProcessingElements for () {
    fn to_processing_elements(self) -> Vec<ProcessorElement> {
        Vec::new()
    }
}

macro_rules! for_tuples {
    (
        $($i:ident),*
    ) => {
        impl<$($i: ProcessingElement + 'static),*> IntoProcessingElements for ($($i),*) {
            #[allow(non_snake_case)]
            fn to_processing_elements(self) -> Vec<ProcessorElement> {
                let mut stack = self.0.stack().to_processing_elements();
                let ($($i),*) = self;
                $(
                    stack.push(ProcessorElement::new($i));
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
