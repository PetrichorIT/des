use core::fmt;
use std::str::FromStr;

///
/// A unqiue identifier for a object, indicating its parental inheritance.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectPath {
    data: String,
    last_element_offset: usize,
    len: usize,
    is_channel: bool,
}

impl ObjectPath {
    /// Indicates whether the path points to the simulation root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.len == 0
    }

    /// Indicates whether the path points to a channel.
    #[must_use]
    pub fn is_channel(&self) -> bool {
        self.is_channel
    }

    /// Indicates whether the path points to a module.
    #[must_use]
    pub fn is_module(&self) -> bool {
        !self.is_channel
    }

    /// Returns the depth of the referenced object.
    ///
    /// Note that depth 0 indicates the root of the simulation.
    #[must_use]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the last path component, the name of the current module.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.data[self.last_element_offset..]
    }

    /// Returns the entrie path as a &str.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.data.as_str()
    }

    /// Returns the entrie path as a &str for declaring a logger scope
    #[must_use]
    pub fn as_logger_scope(&self) -> &str {
        if self.is_root() {
            "@root"
        } else {
            self.as_str()
        }
    }

    /// Returns the entrie path as a &str.
    #[must_use]
    pub fn as_parent_str(&self) -> &str {
        &self.data[..self.last_element_offset.saturating_sub(1)]
    }

    /// Constructs the path to the parent element, if there is any.
    #[must_use]
    pub fn parent(&self) -> Option<ObjectPath> {
        if self.len == 0 {
            return None;
        }

        let mut parent = self.clone();
        parent
            .data
            .truncate(self.last_element_offset.saturating_sub(1));

        if let Some(i) = parent.data.rfind('.') {
            parent.last_element_offset = i + 1;
        } else {
            parent.last_element_offset = 0;
        }
        parent.len -= 1;
        parent.is_channel = false;

        Some(parent)
    }

    /// Returns a parent that is not root.
    pub fn nonzero_parent(&self) -> Option<ObjectPath> {
        let parent = self.parent()?;
        if parent.is_root() {
            None
        } else {
            Some(parent)
        }
    }

    /// Creates a new object path pointing to the root.
    #[must_use]
    pub fn new() -> ObjectPath {
        Self {
            data: String::new(),
            last_element_offset: 0,
            len: 0,
            is_channel: false,
        }
    }

    /// Appends another module to the path.
    ///
    /// # Panics
    ///
    /// This function panics if self allready points to a channel,
    /// since channels are leaf elements in the object tree.
    ///
    pub fn append(&mut self, module: impl AsRef<str>) {
        assert!(
            !self.is_channel,
            "Cannot append to a path that points to a channel"
        );
        let module = module.as_ref();
        if module != "" {
            if self.len != 0 {
                self.last_element_offset = self.data.len() + 1;
                self.data.push('.');
            }

            self.data.push_str(module);
            self.len += 1;
        }
    }

    /// Append a channel leaf to the path.
    ///
    /// # Panics
    ///
    /// This function panics if self is pointing to a channel,
    /// since channels are leaf elements in the object tree.
    ///
    pub fn append_channel(&mut self, channel: impl AsRef<str>) {
        assert!(
            !self.is_channel,
            "Cannot append to a path that points to a channel"
        );
        let channel = channel.as_ref();
        if self.len != 0 {
            self.last_element_offset = self.data.len() + 1;
            self.data.push('.');
        }
        self.data.push('<');
        self.data.push_str(channel);
        self.data.push('>');
        self.is_channel = true;
        self.len += 1;
    }

    /// Returns a new instance with another module appended to the path.
    #[must_use]
    pub fn appended(&self, module: impl AsRef<str>) -> Self {
        let mut clone = self.clone();
        clone.append(module);
        clone
    }

    /// Returns a new instance with a channel appended to the path.
    #[must_use]
    pub fn appended_channel(&self, channel: impl AsRef<str>) -> Self {
        let mut clone = self.clone();
        clone.append_channel(channel);
        clone
    }
}

impl fmt::Display for ObjectPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl AsRef<str> for ObjectPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for ObjectPath {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut o = 0;
        let mut last_element_offset = 0;
        let mut len = 0;
        for c in s.chars() {
            if c == '.' {
                last_element_offset = o + c.len_utf8();
                len += 1;
            }
            o += c.len_utf8();
        }
        if o != last_element_offset {
            len += 1;
        }

        let name = &s[last_element_offset..];
        let is_channel = name.starts_with('<') && name.ends_with('>');

        Ok(Self {
            data: s.to_string(),
            last_element_offset,
            len,
            is_channel,
        })
    }
}

impl From<&str> for ObjectPath {
    fn from(value: &str) -> Self {
        Self::from_str(value).unwrap()
    }
}

impl From<&String> for ObjectPath {
    fn from(value: &String) -> Self {
        Self::from_str(value.as_str()).unwrap()
    }
}

impl From<String> for ObjectPath {
    fn from(value: String) -> Self {
        Self::from_str(value.as_str()).unwrap()
    }
}

impl Default for ObjectPath {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_appending() {
        let mut path = ObjectPath::new();
        path.append("top");
        path.append("mid");
        assert_eq!(path.name(), "mid");
        assert_eq!(path.as_parent_str(), "top");
        assert_eq!(
            path,
            ObjectPath {
                data: "top.mid".to_string(),
                len: 2,
                last_element_offset: 4,
                is_channel: false,
            }
        );

        let mut path = ObjectPath::new();
        path.append("top");
        path.append("mid");
        path.append("low");
        assert_eq!(path.name(), "low");
        assert_eq!(path.as_parent_str(), "top.mid");
        assert_eq!(
            path,
            ObjectPath {
                data: "top.mid.low".to_string(),
                len: 3,
                last_element_offset: 8,
                is_channel: false,
            }
        );

        let mut path = ObjectPath::new();
        path.append("top");
        assert_eq!(path.name(), "top");
        assert_eq!(path.as_parent_str(), "");
        assert_eq!(
            path,
            ObjectPath {
                data: "top".to_string(),
                len: 1,
                last_element_offset: 0,
                is_channel: false,
            }
        );

        let path = ObjectPath::new();
        assert_eq!(path.name(), "");
        assert_eq!(path.as_parent_str(), "");
        assert!(path.is_root());
        assert_eq!(
            path,
            ObjectPath {
                data: "".to_string(),
                len: 0,
                last_element_offset: 0,
                is_channel: false,
            }
        );
    }

    #[test]
    fn parent_creation() {
        let mut path = ObjectPath::new();
        path.append("top");
        path.append("mid");

        let parent = path.parent();
        assert_eq!(
            parent,
            Some(ObjectPath {
                data: "top".to_string(),
                len: 1,
                last_element_offset: 0,
                is_channel: false,
            })
        );

        let mut path = ObjectPath::new();
        path.append("top");
        path.append("mid");
        path.append("low");

        let parent = path.parent();
        assert_eq!(
            parent,
            Some(ObjectPath {
                data: "top.mid".to_string(),
                len: 2,
                last_element_offset: 4,
                is_channel: false,
            })
        );

        let mut path = ObjectPath::new();
        path.append("top");

        let parent = path.parent();
        assert_eq!(
            parent,
            Some(ObjectPath {
                data: "".to_string(),
                len: 0,
                last_element_offset: 0,
                is_channel: false,
            })
        );

        let path = ObjectPath::new();

        let parent = path.parent();
        assert_eq!(parent, None);
    }

    #[test]
    fn parsing() {
        assert_eq!(
            ObjectPath::from("top.mid"),
            ObjectPath {
                data: "top.mid".to_string(),
                len: 2,
                last_element_offset: 4,
                is_channel: false,
            }
        );

        assert_eq!(
            ObjectPath::from("top.mid.low"),
            ObjectPath {
                data: "top.mid.low".to_string(),
                len: 3,
                last_element_offset: 8,
                is_channel: false,
            }
        );

        assert_eq!(
            ObjectPath::from("top"),
            ObjectPath {
                data: "top".to_string(),
                len: 1,
                last_element_offset: 0,
                is_channel: false,
            }
        );

        assert_eq!(
            ObjectPath::from(""),
            ObjectPath {
                data: "".to_string(),
                len: 0,
                last_element_offset: 0,
                is_channel: false,
            }
        );

        // emoji is a 4 byte character thus 7 + 4
        assert_eq!(
            ObjectPath::from("top.a😀b.low"),
            ObjectPath {
                data: "top.a😀b.low".to_string(),
                len: 3,
                last_element_offset: 11,
                is_channel: false,
            }
        );
    }

    #[test]
    fn channel() {
        assert_eq!(
            ObjectPath::from("top.<low>"),
            ObjectPath {
                data: "top.<low>".to_string(),
                len: 2,
                last_element_offset: 4,
                is_channel: true,
            }
        );

        let path = ObjectPath::from("top.<low>");
        assert!(path.is_channel());
        assert_eq!(
            path.parent(),
            Some(ObjectPath {
                data: "top".to_string(),
                last_element_offset: 0,
                len: 1,
                is_channel: false
            })
        );
    }

    #[test]
    #[should_panic]
    fn channel_append_something() {
        let mut path = ObjectPath::from("top.<low>");
        path.append("mod");
    }
}
