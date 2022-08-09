use std::{error::Error, fmt::Display, str::FromStr};

///
/// A unqiue identifier for a object, indicating its parental inheritance.
///
/// The format is the following:
/// subsys/subsys/subsys.module.module#channel
///
/// The storage format follows this convention.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectPath {
    data: String,
    module_offset: usize,
    channel_offset: usize,
    last_element_offset: usize,
    // There are three ptrs:
    // module_offset, channel_offset, last_element_offset:
    //
    // Self is a subsystem IF:
    // - last_element_offset < module_offset
    // -> module_offset must be after the last element so module_offset == len()
    //
    // Self is a module IF:
    // - module_offset <= last_element offset
    //   and last_element_offset < channel_offset
    //   since that means channel_offset is len()
    //
    // Self is channel IF
    // channel_offset == last_element_offset
    //
    // NOTe channel_offset < last_element_offset is invalid.
}

impl ObjectPath {
    ///
    /// Indicates whether the pointee is a channel.
    ///
    #[must_use] pub fn is_channel(&self) -> bool {
        self.last_element_offset == self.channel_offset
    }

    ///
    /// Indicates whether the pointee to object is
    /// a subsystem.
    ///
    #[must_use] pub fn is_subsystem(&self) -> bool {
        self.last_element_offset < self.module_offset
            && self.last_element_offset < self.channel_offset
    }

    ///
    /// Indicates whether the pointee to object is
    /// a module.
    ///
    #[must_use] pub fn is_module(&self) -> bool {
        !self.is_subsystem() && !self.is_channel()
    }

    ///
    /// Returns the local name of the pointee,
    /// aka. the last path element.
    ///
    #[must_use] pub fn name(&self) -> &str {
        &self.data[self.last_element_offset..]
    }

    ///
    /// Returns the full path to the pointee.
    ///
    #[must_use] pub fn path(&self) -> &str {
        &self.data
    }

    ///
    /// Returns a pointer to that parent entity
    /// or `None` if self is a root entity.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use des::prelude::*;
    /// let path = ObjectPath::new("MainNet/SubNet.Module.Submodule".to_string()).unwrap();
    ///
    /// assert_eq!(
    ///     path.parent(),
    ///     Some(ObjectPath::new("MainNet/SubNet.Module".to_string()).unwrap())
    /// );
    /// assert_eq!(
    ///     path.parent().unwrap().parent(),
    ///     Some(ObjectPath::new("MainNet/SubNet".to_string()).unwrap())
    /// );
    /// assert_eq!(
    ///     path.parent().unwrap().parent().unwrap().parent(),
    ///     Some(ObjectPath::new("MainNet".to_string()).unwrap())
    /// );
    /// assert_eq!(
    ///     path.parent().unwrap().parent().unwrap().parent().unwrap().parent(),
    ///     None
    /// );
    /// ```
    ///
    #[must_use] pub fn parent(&self) -> Option<ObjectPath> {
        if self.is_subsystem() {
            // find last slash in the set
            if self.last_element_offset != 0 {
                let data = self.data[..(self.last_element_offset - 1)].to_string();
                let last_element_offset = data.rfind('/').unwrap_or(0);
                Some(ObjectPath {
                    channel_offset: data.len(),
                    module_offset: data.len(),
                    last_element_offset,
                    data,
                })
            } else {
                // Current element is root
                None
            }
        } else if self.is_module() {
            // Check edge case last module is deleted
            let next_delim = if self.module_offset == self.last_element_offset {
                '/'
            } else {
                '.'
            };

            if self.last_element_offset != 0 {
                let data = self.data[..(self.last_element_offset - 1)].to_string();
                let last_element_offset = data.rfind(next_delim).map_or(0, |v| v + 1);
                let module_offset = if next_delim == '/' {
                    data.len()
                } else {
                    self.module_offset
                };

                Some(ObjectPath {
                    module_offset,
                    last_element_offset,
                    channel_offset: data.len(),
                    data,
                })
            } else {
                // Current element is root
                None
            }
        } else {
            // Self is a channel.
            let next_delim = if self.module_offset >= self.last_element_offset {
                '/'
            } else {
                '.'
            };

            if self.last_element_offset != 0 {
                let data = self.data[..(self.last_element_offset - 1)].to_string();
                let last_element_offset = data.rfind(next_delim).map_or(0, |v| v + 1);
                let module_offset = if next_delim == '/' {
                    data.len()
                } else {
                    self.module_offset
                };

                Some(ObjectPath {
                    module_offset,
                    last_element_offset,
                    channel_offset: data.len(),
                    data,
                })
            } else {
                // Current element is root
                None
            }
        }
    }

    ///
    /// Returns the part of the path that does not
    /// include the pointees name.
    ///
    #[must_use] pub fn parent_path(&self) -> &str {
        if self.last_element_offset == 0 {
            &self.data[..0]
        } else {
            &self.data[..(self.last_element_offset - 1)]
        }
    }

    ///
    /// Creates a new pointer given a raw string description of
    /// the pointer.
    ///
    /// This operation may fail if:
    /// - the provided string is empty
    /// - the string contains a path element with no name (e.g. "Main/.Module")
    /// - the string describes modules within the subsystem part of the path (e.g. "Main.Subystem/Subsystem.Module")
    ///
    /// # Example
    ///
    /// ```rust
    /// # use des::prelude::*;
    /// let path = ObjectPath::new("Main/Subnet.Module.Submodule".to_string())
    ///     .expect("This should not fail");
    /// assert!(path.is_module());
    /// assert_eq!(path.path(), "Main/Subnet.Module.Submodule");
    /// assert_eq!(path.parent_path(), "Main/Subnet.Module");
    /// assert_eq!(path.name(), "Submodule");
    ///
    /// let empty_path = ObjectPath::new("".to_string());
    /// assert!(empty_path.is_err());
    ///
    /// let empty_element = ObjectPath::new("Main/.Module".to_string());
    /// assert!(empty_element.is_err());
    ///
    /// let unordered = ObjectPath::new("Main.Subystem/Subsystem.Module".to_string());
    /// assert!(unordered.is_err());
    /// ```
    #[must_use]
    pub fn new(data: String) -> Result<Self, ObjectPathParseError> {
        if data.is_empty() {
            return Err(ObjectPathParseError::EmptyPath);
        }

        let dot_left = data.find('.');
        let dot_right = data.rfind('.');
        // let slash_left = data.find('/');
        let slash_right = data.rfind('/');

        let shbang = data.rfind('#');

        let module_offset = dot_left.map_or(data.len(), |v| v + 1);
        let mut last_element_offset = if dot_right.is_some() {
            dot_right.map_or(0, |v| v + 1)
        } else {
            slash_right.map_or(0, |v| v + 1)
        };

        // Check interity
        if let (Some(dot_left), Some(slash_right)) = (dot_left, slash_right) {
            if dot_left < slash_right {
                return Err(ObjectPathParseError::UnorderedPath);
            }
        }

        // Check all path elements
        let mut acc = 0;
        for c in data.chars() {
            if c == '.' || c == '/' {
                if acc == 0 {
                    return Err(ObjectPathParseError::EmptyPathElement);
                }
                acc = 0;
            } else {
                acc += 1;
            }
        }

        // Catch final delims
        if acc == 0 {
            return Err(ObjectPathParseError::EmptyPathElement);
        }

        let channel_offset = if let Some(shbang) = shbang {
            last_element_offset = shbang + 1;
            shbang + 1
        } else {
            data.len()
        };

        Ok(ObjectPath { data, module_offset, channel_offset, last_element_offset })
    }

    ///
    /// Creates a new pointer to a top-level subsystem.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use des::prelude::*;
    /// let path = ObjectPath::root_subsystem("MyNetwork".to_string());
    ///
    /// assert!(path.is_subsystem());
    /// assert_eq!(path.path(), "MyNetwork");
    /// assert_eq!(path.parent_path(), "");
    /// assert_eq!(path.name(), "MyNetwork");
    /// assert_eq!(path.parent(), None);
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics if the provided name does contain
    /// seperation characters like '.' or '/'.
    ///
    /// ```should_panic
    /// # use des::prelude::*;
    /// let path = ObjectPath::root_subsystem("MyNetwork/Subnet.Module".to_string());
    /// ```
    ///
    #[must_use]
    pub fn root_subsystem(name: String) -> Self {
        assert!(!name.contains('/') && !name.contains('.'));

        Self {
            module_offset: name.len(),
            channel_offset: name.len(),
            last_element_offset: 0,
            data: name,
        }
    }

    ///
    /// Creates a new pointer to a top-level module.
    /// This function cannot be used together with ndl
    /// since ndl requires a top-level subsystem.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use des::prelude::*;
    /// let path = ObjectPath::root_module("MyModule".to_string());
    ///
    /// assert!(path.is_module());
    /// assert_eq!(path.path(), "MyModule");
    /// assert_eq!(path.parent_path(), "");
    /// assert_eq!(path.name(), "MyModule");
    /// assert_eq!(path.parent(), None);
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics if the provided name does contain
    /// seperation characters like '.' or '/'.
    ///
    /// ```should_panic
    /// # use des::prelude::*;
    /// let path = ObjectPath::root_module("MyModule.SubModule".to_string());
    /// ```
    ///
    #[must_use]
    pub fn root_module(name: String) -> Self {
        assert!(!name.contains('/') && !name.contains('.'));

        Self {
            channel_offset: name.len(),
            data: name,
            last_element_offset: 0,
            module_offset: 0,
        }
    }

    ///
    /// Creates a new channel ident.
    ///
    #[must_use]
    pub fn channel_with(channel: &str, parent: &ObjectPath) -> Self {
        assert!(!parent.is_channel());

        let data = format!("{}#{}", parent.data, channel);
        let last_element_offset = parent.data.len() + 1;

        Self {
            data,
            last_element_offset,
            module_offset: parent.module_offset,
            channel_offset: last_element_offset,
        }
    }

    ///
    /// Creates a new pointer to a subsystem attached to a
    /// parent subsystem.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use des::prelude::*;
    /// let parent = ObjectPath::root_subsystem("Main".to_string());
    /// let child = ObjectPath::subsystem_with_parent("Subnet", &parent);
    ///
    /// assert!(child.is_subsystem());
    /// assert_eq!(child.path(), "Main/Subnet");
    /// assert_eq!(child.parent_path(), "Main");
    /// assert_eq!(child.name(), "Subnet");
    /// assert_eq!(child.parent(), Some(parent));
    /// ```
    ///
    /// # Panics
    ///
    /// This functions panics should the provided name contain
    /// seperator characters OR should the provided parent not be
    /// a submodule pointer.
    ///
    /// ```should_panic
    /// # use des::prelude::*;
    /// let parent = ObjectPath::root_module("Main".to_string());
    /// let child = ObjectPath::subsystem_with_parent("Subnet", &parent);
    /// ```
    ///
    #[must_use]
    pub fn subsystem_with_parent(name: &str, parent: &ObjectPath) -> Self {
        assert!(!name.contains('/') && !name.contains('.'));

        assert!(parent.is_subsystem());
        let data = format!("{}/{}", parent.data, name);
        let last_element_offset = parent.data.len() + 1;

        Self {
            module_offset: data.len(),
            channel_offset: data.len(),
            last_element_offset,
            data,
        }
    }

    ///
    /// Creates a new pointer to a module attached to some
    /// parent entity.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use des::prelude::*;
    /// let parent = ObjectPath::root_subsystem("Main".to_string());
    /// let child = ObjectPath::module_with_parent("Module", &parent);
    ///
    /// assert!(child.is_module());
    /// assert_eq!(child.path(), "Main.Module");
    /// assert_eq!(child.parent_path(), "Main");
    /// assert_eq!(child.name(), "Module");
    /// assert_eq!(child.parent(), Some(parent));
    /// ```
    ///
    /// # Panics
    ///
    /// This functions panics should the provided name contain
    /// seperator characters.
    ///
    #[must_use]
    pub fn module_with_parent(name: &str, parent: &ObjectPath) -> Self {
        assert!(!name.contains('/') && !name.contains('.'));
        assert!(!parent.is_channel());

        let data = format!("{}.{}", parent.data, name);
        let last_element_offset = parent.data.len() + 1;

        let module_offset = if parent.is_module() {
            parent.module_offset
        } else {
            last_element_offset
        };

        Self {
            channel_offset: data.len(),
            data,
            module_offset,
            last_element_offset,
        }
    }
}

impl Display for ObjectPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl FromStr for ObjectPath {
    type Err = ObjectPathParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

impl AsRef<str> for ObjectPath {
    fn as_ref(&self) -> &str {
        self.data.as_ref()
    }
}

/// An error that has occured upon parsing a String to a `ObjectPath`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectPathParseError {
    /// The provided string is empty.
    EmptyPath,
    /// The provided string contains a path element width width 0.
    EmptyPathElement,
    /// The provided string does not contain a path in the form ([subsys]/+.)?[module].+
    UnorderedPath,
}

impl Display for ObjectPathParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => write!(f, "Cannot create 'ObjectPath' from an empty string."),
            Self::EmptyPathElement => write!(
                f,
                "Cannot create 'ObjectPathElement' with an empty path element."
            ),
            Self::UnorderedPath => {
                write!(
                    f,
                    "Cannot create 'ObjectPath' from invalid unorderd string."
                )
            }
        }
    }
}
impl Error for ObjectPathParseError {}
