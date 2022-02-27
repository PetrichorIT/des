use crate::{net::common::Parameter, *};

create_global_uid!(
    /// A runtime-unqiue identifier for a module / submodule inheritence tree.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub ModuleId(u16) = MODULE_ID;
);

///
/// The usecase independent core of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone)]
pub struct ModuleCore {
    id: ModuleId,

    /// A human readable identifier for the module.
    pub path: ModulePath,

    /// A collection of all gates register to the current module
    pub gates: Vec<GateRef>,

    /// A buffer of messages to be send out, after the current handle messsage terminates.
    pub out_buffer: Vec<(Message, GateId)>,

    /// A buffer of wakeup calls to be enqueued, after the current handle message terminates.
    pub loopback_buffer: Vec<(Message, SimTime)>,

    /// The period of the activity coroutine (if zero than there is no coroutine).
    pub activity_period: SimTime,

    /// An indicator whether a valid activity timeout is existent.
    pub activity_active: bool,

    /// The module identificator for the parent module.
    pub parent_ptr: Option<*mut u8>,

    /// A set of local parameters
    pub parameters: Vec<Parameter>,
}

impl ModuleCore {
    /// A runtime specific but unqiue identifier for a given module.
    #[inline(always)]
    pub fn id(&self) -> ModuleId {
        self.id
    }

    /// A human readable identifer for a given module.
    pub fn identifier(&self) -> &str {
        self.path.module_path()
    }

    ///
    /// Creates a new optionally named instance
    /// of 'Self'.
    ///
    pub fn new_with(path: ModulePath) -> Self {
        Self {
            id: ModuleId::gen(),
            gates: Vec::new(),
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
            parent_ptr: None,
            path,
            parameters: Vec::new(),
        }
    }

    ///
    /// Creates  a not-named instance of 'Self'.
    ///
    #[inline(always)]
    pub fn new() -> Self {
        Self::new_with(ModulePath::root(String::from("unknown-module")))
    }
}

impl Default for ModuleCore {
    fn default() -> Self {
        Self::new()
    }
}
