use std::{
    any::{type_name, TypeId},
    collections::HashMap,
    error::Error,
    fmt::Display,
};

use crate::{
    net::common::Parameters,
    util::spmc::{SpmcReader, SpmcWriter},
    *,
};

create_global_uid!(
    /// A runtime-unqiue identifier for a module / submodule inheritence tree.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub ModuleId(u16) = MODULE_ID;
);

type TypedModulePtr = (*mut u8, TypeId);

///
/// The usecase independent core of a module.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone)]
pub struct ModuleCore {
    id: ModuleId,

    /// A human readable identifier for the module.
    pub(crate) path: ModulePath,

    /// A collection of all gates register to the current module
    pub(crate) gates: Vec<GateRef>,

    /// A buffer of messages to be send out, after the current handle messsage terminates.
    pub(crate) out_buffer: Vec<(Message, GateRef)>,

    /// A buffer of wakeup calls to be enqueued, after the current handle message terminates.
    pub(crate) loopback_buffer: Vec<(Message, SimTime)>,

    /// The period of the activity coroutine (if zero than there is no coroutine).
    pub(crate) activity_period: SimTime,

    /// An indicator whether a valid activity timeout is existent.
    pub(crate) activity_active: bool,

    /// The module identificator for the parent module.
    parent_ptr: Option<TypedModulePtr>,

    /// The collection of child nodes for the curretn module.
    childern: HashMap<String, TypedModulePtr>,

    /// A set of local parameters.
    ///
    /// TODO: Restrict to pub(crate) if possible providing constructors to created valid subinstances.
    parameters: SpmcReader<Parameters>,
}

impl ModuleCore {
    ///
    /// A runtime-unqiue identifier for this module-core and by extension this module.
    ///
    pub fn id(&self) -> ModuleId {
        self.id
    }

    ///
    /// Creates a new optionally named instance
    /// of 'Self'.
    ///
    pub fn new_with(path: ModulePath, parameters: SpmcReader<Parameters>) -> Self {
        Self {
            id: ModuleId::gen(),
            gates: Vec::new(),
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
            parent_ptr: None,
            path,
            childern: HashMap::new(),
            parameters,
        }
    }

    ///
    /// Creates a new module core based on the parent
    /// using the name to extend the path.
    ///
    pub fn child_of(name: &str, parent: &ModuleCore) -> Self {
        let path = ModulePath::new_with_parent(name, &parent.path);

        Self {
            id: ModuleId::gen(),
            path,
            gates: Vec::new(),
            out_buffer: Vec::new(),
            loopback_buffer: Vec::new(),
            activity_period: SimTime::ZERO,
            activity_active: false,
            parent_ptr: None,
            childern: HashMap::new(),
            parameters: parent.parameters.clone(),
        }
    }

    ///
    /// Creates  a not-named instance of 'Self'.
    ///
    #[inline(always)]
    pub fn new() -> Self {
        Self::new_with(
            ModulePath::root(String::from("unknown-module")),
            SpmcWriter::new(Parameters::new()).get_reader(),
        )
    }
}

///
/// # Child / Parent management
///

impl ModuleCore {
    ///
    /// Adds a child module to the current module, registering self as the parent
    /// for the child module.
    ///
    pub fn add_child<SelfModuleType, T>(&mut self, module: &mut T)
    where
        SelfModuleType: 'static,
        T: 'static + StaticModuleCore,
    {
        // Set parent ptr for child
        let self_ptr: *mut Self = &mut *self;
        let self_ptr: *mut u8 = self_ptr as *mut u8;
        module.module_core_mut().parent_ptr = Some((self_ptr, TypeId::of::<SelfModuleType>()));

        let child_name = module.name().to_string();

        // Add to child ptr list.
        let child_ptr: *mut T = &mut *module;
        let child_ptr: *mut u8 = child_ptr as *mut u8;
        self.childern
            .insert(child_name, (child_ptr, TypeId::of::<T>()));
    }

    ///
    /// Returns a reference to a typed child module if a) the child module exists under the given
    /// name, and b) the child module is of type T.
    ///
    pub fn child<T>(&self, name: &str) -> Result<&T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        let ptr = self.check_reference_intergrity(self.childern.get(name).copied())?;
        unsafe { Ok(&*ptr) }
    }

    ///
    /// Returns a mutable reference to a typed child module if a) the child module exists under the given
    /// name, and b) the child module is of type T.
    ///
    pub fn child_mut<T>(&mut self, name: &str) -> Result<&mut T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        let ptr = self.check_reference_intergrity(self.childern.get(name).copied())?;
        unsafe { Ok(&mut *ptr) }
    }

    ///
    /// Returns a reference to a typed parent module if a) this module has a parent,
    /// and b) the parent module is of type T.
    ///
    pub fn parent<T>(&self) -> Result<&T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        let ptr = self.check_reference_intergrity(self.parent_ptr)?;
        unsafe { Ok(&*ptr) }
    }

    ///
    /// Returns a mutable reference to a typed parent module if a) this module has a parent,
    /// and b) the parent module is of type T.
    ///
    pub fn parent_mut<T>(&mut self) -> Result<&mut T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        let ptr = self.check_reference_intergrity(self.parent_ptr)?;
        unsafe { Ok(&mut *ptr) }
    }

    ///
    /// Checks a ptr according to the error parameter define on ModuleReferencingError.
    ///
    /// Note that self is not nessecary in this method, just for call conveinice.
    ///
    fn check_reference_intergrity<T>(
        &self,
        ptr: Option<TypedModulePtr>,
    ) -> Result<*mut T, ModuleReferencingError>
    where
        T: 'static + StaticModuleCore,
    {
        match ptr {
            Some((ptr, typ)) => {
                if typ == TypeId::of::<T>() {
                    Ok(ptr as *mut T)
                } else {
                    Err(ModuleReferencingError::TypeError(format!(
                        "Parent module is not of type {}.",
                        type_name::<T>()
                    )))
                }
            }
            None => Err(ModuleReferencingError::NoPtrUnderThatName(String::from(
                "This module does not posses a parent ptr.",
            ))),
        }
    }
}

///
/// # Parameter management
///

impl ModuleCore {
    ///
    /// Returns the parameters for the current module.
    ///
    pub fn pars(&self) -> HashMap<String, String> {
        self.parameters.get(self.path.path())
    }

    ///
    /// Returns a reference to the parameter store, used for constructing
    /// custom instances of modules.
    ///
    pub fn pars_ref(&self) -> SpmcReader<Parameters> {
        self.parameters.clone()
    }
}

impl Default for ModuleCore {
    fn default() -> Self {
        Self::new()
    }
}

///
/// An error while resolving a reference to another module.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleReferencingError {
    NoPtrUnderThatName(String),
    TypeError(String),
}

impl Display for ModuleReferencingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPtrUnderThatName(str) => write!(f, "{}", str),
            Self::TypeError(str) => write!(f, "{}", str),
        }
    }
}

impl Error for ModuleReferencingError {}
