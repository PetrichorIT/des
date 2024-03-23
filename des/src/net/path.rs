use core::fmt;
use std::sync::Arc;

///
/// A unqiue identifier for a object, indicating its parental inheritance.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectPath {
    data: Arc<str>,
    last_element_offset: usize,
    len: usize,
    is_gate: bool,
}

impl ObjectPath {
    /// Indicates whether the path points to the simulation root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.len == 0
    }

    /// Indicates whether the path points to a module.
    #[must_use]
    pub fn is_module(&self) -> bool {
        !self.is_gate
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
        &self.data
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

        let mut data = self.data.to_string();
        let mut last_element_offset = self.last_element_offset;
        let mut len = self.len;

        data.truncate(last_element_offset.saturating_sub(1));

        if let Some(i) = data.rfind('.') {
            last_element_offset = i + 1;
        } else {
            last_element_offset = 0;
        }
        len -= 1;

        Some(Self {
            data: data.into(),
            last_element_offset,
            len,
            is_gate: false,
        })
    }

    /// Returns a parent that is not root.
    #[must_use]
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
            data: String::new().into(),
            last_element_offset: 0,
            len: 0,
            is_gate: false,
        }
    }

    /// Returns a new instance with another module appended to the path.
    ///
    /// # Panics
    ///
    /// This function panics if the current path points to a gate.
    #[must_use]
    pub fn appended(&self, module: impl AsRef<str>) -> Self {
        let mut data = self.data.to_string();
        let mut last_element_offset = self.last_element_offset;
        let mut len = self.len;

        assert!(
            !self.is_gate,
            "cannot append to a path that points to a gate"
        );

        let suffix = module.as_ref();
        if !suffix.is_empty() {
            if self.len != 0 {
                last_element_offset = data.len() + 1;
                data.push('.');
            }
            data.push_str(suffix);
            len += 1;
        }

        Self {
            data: data.into(),
            last_element_offset,
            len,
            is_gate: false,
        }
    }

    /// Retruns a new object path pointing to the gate on the current module.
    pub fn appended_gate(&self, gate: impl AsRef<str>) -> Self {
        let mut appended = self.appended(gate);
        appended.is_gate = true;
        appended
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

impl From<&str> for ObjectPath {
    fn from(s: &str) -> Self {
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

        Self {
            data: s.to_string().into(),
            last_element_offset,
            len,
            is_gate: false,
        }
    }
}

impl From<&String> for ObjectPath {
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<String> for ObjectPath {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
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
        let path = ObjectPath::new().appended("top").appended("mid");

        assert_eq!(path.name(), "mid");
        assert_eq!(path.as_parent_str(), "top");
        assert_eq!(
            path,
            ObjectPath {
                data: "top.mid".to_string().into(),
                len: 2,
                last_element_offset: 4,
                is_gate: false,
            }
        );

        let path = ObjectPath::new()
            .appended("top")
            .appended("mid")
            .appended("low");

        assert_eq!(path.name(), "low");
        assert_eq!(path.as_parent_str(), "top.mid");
        assert_eq!(
            path,
            ObjectPath {
                data: "top.mid.low".to_string().into(),
                len: 3,
                last_element_offset: 8,
                is_gate: false,
            }
        );

        let path = ObjectPath::new().appended("top");
        assert_eq!(path.name(), "top");
        assert_eq!(path.as_parent_str(), "");
        assert_eq!(
            path,
            ObjectPath {
                data: "top".to_string().into(),
                len: 1,
                last_element_offset: 0,
                is_gate: false,
            }
        );

        let path = ObjectPath::new();
        assert_eq!(path.name(), "");
        assert_eq!(path.as_parent_str(), "");
        assert!(path.is_root());
        assert_eq!(
            path,
            ObjectPath {
                data: "".to_string().into(),
                len: 0,
                last_element_offset: 0,
                is_gate: false,
            }
        );
    }

    #[test]
    fn parent_creation() {
        let path = ObjectPath::new().appended("top").appended("mid");
        let parent = path.parent();
        assert_eq!(
            parent,
            Some(ObjectPath {
                data: "top".to_string().into(),
                len: 1,
                last_element_offset: 0,
                is_gate: false,
            })
        );

        let path = ObjectPath::new()
            .appended("top")
            .appended("mid")
            .appended("low");

        let parent = path.parent();
        assert_eq!(
            parent,
            Some(ObjectPath {
                data: "top.mid".to_string().into(),
                len: 2,
                last_element_offset: 4,
                is_gate: false,
            })
        );

        let path = ObjectPath::new().appended("top");

        let parent = path.parent();
        assert_eq!(
            parent,
            Some(ObjectPath {
                data: "".to_string().into(),
                len: 0,
                last_element_offset: 0,
                is_gate: false,
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
                data: "top.mid".to_string().into(),
                len: 2,
                last_element_offset: 4,
                is_gate: false,
            }
        );

        assert_eq!(
            ObjectPath::from("top.mid.low"),
            ObjectPath {
                data: "top.mid.low".to_string().into(),
                len: 3,
                last_element_offset: 8,
                is_gate: false,
            }
        );

        assert_eq!(
            ObjectPath::from("top"),
            ObjectPath {
                data: "top".to_string().into(),
                len: 1,
                last_element_offset: 0,
                is_gate: false,
            }
        );

        assert_eq!(
            ObjectPath::from(""),
            ObjectPath {
                data: "".to_string().into(),
                len: 0,
                last_element_offset: 0,
                is_gate: false,
            }
        );

        // emoji is a 4 byte character thus 7 + 4
        assert_eq!(
            ObjectPath::from("top.a😀b.low"),
            ObjectPath {
                data: "top.a😀b.low".to_string().into(),
                len: 3,
                last_element_offset: 11,
                is_gate: false,
            }
        );
    }
}
