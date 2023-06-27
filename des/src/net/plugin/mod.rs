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

use crate::prelude::Message;
use std::any::Any;

mod api;
pub use self::api::{add_plugin, get_plugin_state, PluginHandle};

mod error;
pub use self::error::{PluginError, PluginErrorKind};

mod registry;
pub(crate) use self::registry::PluginRegistry;
pub use self::registry::PluginStatus;

use super::module::with_mod_ctx;

/// A subprogramm between the module application and the network layer.
///
/// Plugins can follow different patterns based on the provided
/// API. Common patterns are:
///
/// - **Observer**: The plugin does not modifiy the message stream, it just observes it.
///    This plugin can be used to get statistics over message streams or to log
///    debug output.
/// - **Scope-Provider**: This plugin provides some kind of scope to all items further
///    from the network layer than itself. A scope can be defined using a static variable
///    or just consist of a time meassurement between
///    [`event_start`](Plugin::event_start) / [`event_end`](Plugin::event_end).
/// - **Capture**: This kind of plugins captures parts of the input stream and redirects
///     it in some abitraty way, using other APIs. This pattern can be used to implement buffering
///     or mergeing of frameneted IP packets.
/// - **Meta-Provider**: This kind of plugin attaches / modifies part of the incoming or
///    outgoing message stream to provide some new level of abstraction e.g. a VPN
///    or simulated network Interfaces.
pub trait Plugin: 'static {
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
    /// # use des::net::plugin::*;
    /// # use des::prelude::*;
    /// struct LoggerPlugin {
    ///     counter: usize,
    /// }
    ///
    /// impl Plugin for LoggerPlugin {
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
    /// is the reverse to the call order of
    /// [`event_start`](Plugin::event_start).
    ///
    /// Use this function to set up actions, associated
    /// with the end of an event
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use des::net::plugin::*;
    /// # use des::prelude::*;
    /// # use des::time::*;
    /// struct Timer {
    ///     started: SimTime,    
    /// }
    ///
    /// impl Plugin for Timer {
    ///     fn event_start(&mut self) {
    ///        self.started = SimTime::now();
    ///     }
    ///     fn event_end(&mut self) {
    ///        let t = SimTime::now().duration_since(self.started);
    ///        tracing::trace!("took {:?}", t);   
    ///     }
    /// }
    /// ```
    fn event_end(&mut self) {}

    /// A capture clause that can modify an incoming message.
    ///
    /// This function is called at most once per event, after all
    /// plugins have called
    /// [`event_start`](Plugin::event_start),
    /// but before all the main application has processed its message.
    ///
    /// This function receives an incoming message, and can
    /// modify, pass-through or delete a message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::net::plugin::*;
    /// struct Filter {
    ///    filter: Box<dyn Fn(&Message) -> bool>,    
    /// }
    ///
    /// impl Plugin for Filter {
    ///     fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
    ///        let f = &self.filter;
    ///        if f(&msg) {
    ///            Some(msg)    
    ///        } else {
    ///            None
    ///        }
    ///     }    
    /// }
    /// ```
    fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }

    /// A capture clause that can modify an outgoing mesesage stream.
    ///
    /// This function is called once per message send, thus it can be
    /// called in all parts of the event processing. However
    /// this function is never called before the
    /// [`event_end`](Plugin::event_end) function
    /// of this plugin, but allways after the
    /// [`event_start`](Plugin::event_start) function.
    ///
    /// This function receives outgoing messages which it can modify
    /// delete or passthrough.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::net::plugin::*;
    /// struct RandomizeId;
    ///
    /// impl Plugin for RandomizeId {
    ///     fn capture_outgoing(&mut self, mut msg: Message) -> Option<Message> {
    ///         msg.header_mut().id = random();
    ///         Some(msg)    
    ///     }
    /// }
    /// ```
    fn capture_outgoing(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }

    /// Returns the state of the plugin.
    fn state(&self) -> Box<dyn Any> {
        Box::new(())
    }
}

// # Internals

/// Call this function when a message is send (via send / schedule_*)
/// to process the plugin output stream accordingly.
///
/// # Notes
///
/// Can be called from other plugins, also from plugin upstream or downstream
///
pub(crate) fn plugin_output_stream(msg: Message) -> Option<Message> {
    // (0) Create new downstream
    with_mod_ctx(|ctx| {
        {
            ctx.plugins.write().begin_sub_downstream(None);
        }

        // (1) Move the message allong the downstream, only using active plugins.
        //
        // 3 cases:
        // - call origin is in upstream-plugin (good since all plugins below are ::running with a core stored)
        // - call origin is main (good since all plugins are ::running with a core stored)
        // - call origin is in downstream branch
        //      - good since all plugins below are still ::running with a core and all aboth will be ignored ::idle
        //      - self is not an issue, since without a core not in itr
        //      - BUT: begin_downstream poisoined the old downstream info.
        let mut msg = msg;
        while let Some(mut plugin) = {
            let mut lock = ctx.plugins.write();
            let ret = lock.next_downstream();
            drop(lock);
            ret
        } {
            // (2) Capture the packet

            let rem_msg = plugin.capture_outgoing(msg);

            // (3) Continue iteration if possible
            let mut plugins = ctx.plugins.write();

            plugins.put_back_downstream(plugin, false);
            if let Some(rem_msg) = rem_msg {
                msg = rem_msg;
            } else {
                plugins.close_sub_downstream();
                return None;
            }
        }

        ctx.plugins.write().close_sub_downstream();

        // (4) If the message survives, good.
        Some(msg)
    })
}
