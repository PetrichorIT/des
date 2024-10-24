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

use std::{any::Any, ops::Deref};

use super::{module::Module, util::NoDebug};
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

impl<T: Module> ProcessingElement for T {
    fn incoming(&mut self, msg: Message) -> Option<Message> {
        self.handle_message(msg);
        None
    }
}

/// A untyped set of processing elements, effectivly a processing stack.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Processor {
    // last element is module
    pub(super) state: ProcessingState,
    stack: ProcessingStack,
    pub(super) handler: Box<dyn Module>,
}

impl Processor {
    pub(super) fn new(stack: ProcessingStack, handler: impl Module) -> Self {
        Processor {
            state: ProcessingState::Upstream(0),
            stack,
            handler: Box::new(handler),
        }
    }

    pub(super) fn incoming_upstream(&mut self, msg: Option<Message>) -> Option<Message> {
        self.state = ProcessingState::Upstream(0);

        let mut msg = msg;
        for i in 0..self.stack.items.len() {
            self.stack.items[i].event_start();
            if let Some(existing_msg) = msg {
                msg = self.stack.items[i].incoming(existing_msg);
            }
            self.state.bump_upstream();
        }
        msg
    }

    pub(super) fn incoming_downstream(&mut self) {
        self.state = ProcessingState::Downstream(self.stack.items.len());
        for i in (0..self.stack.items.len()).rev() {
            self.stack.items[i].event_end();
            self.state.bump_downstream();
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

/// A stack of processing elements
#[derive(Debug, Default)]
pub struct ProcessingStack {
    items: NoDebug<Vec<Box<dyn ProcessingElement>>>,
}

impl ProcessingStack {
    /// Merge a new stack onto the the current one.
    pub fn append(&mut self, expansion: impl Into<ProcessingStack>) {
        self.items.extend(expansion.into().items.into_inner());
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

impl<P: ProcessingElement> From<P> for ProcessingStack {
    fn from(value: P) -> Self {
        let boxed: Box<dyn ProcessingElement> = Box::new(value);
        ProcessingStack {
            items: vec![boxed].into(),
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
                let mut stack = ProcessingStack::default();
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
