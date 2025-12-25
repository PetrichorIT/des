use core::fmt;
use std::sync::Arc;

///
/// A unqiue identifier for a object, indicating its parental inheritance.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectPath {
    data: Arc<str>,
    is_gate: bool,
}

impl ObjectPath {
    /// Indicates whether the path points to the simulation root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.data.is_empty()
    }

    /// Indicates whether the path points to a module.
    #[must_use]
    pub fn is_module(&self) -> bool {
        !self.is_gate
    }

    /// Returns the last path component, the name of the current module.
    #[must_use]
    pub fn name(&self) -> &str {
        let last = self.data.rfind('.').map_or(0, |i| i + 1);
        &self.data[last..]
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
        let last = self.data.rfind('.').unwrap_or(0);
        &self.data[..last]
    }

    /// Constructs the path to the parent element, if there is any.
    #[must_use]
    pub fn parent(&self) -> Option<ObjectPath> {
        let path = self.as_parent_str();
        if path.is_empty() {
            None
        } else {
            Some(ObjectPath {
                data: path.into(),
                is_gate: false,
            })
        }
    }

    /// Returns a parent that is not root.
    #[must_use]
    pub fn nonzero_parent(&self) -> Option<ObjectPath> {
        let parent = self.parent()?;
        if parent.is_root() { None } else { Some(parent) }
    }

    /// Returns a new instance with another module appended to the path.
    ///
    /// # Panics
    ///
    /// This function panics if the current path points to a gate.
    #[must_use]
    pub fn appended(&self, module: impl AsRef<str>) -> Self {
        assert!(
            !self.is_gate,
            "cannot append to a path that points to a gate"
        );

        let mut data = self.data.to_string();
        let suffix = module.as_ref();
        if !suffix.is_empty() {
            if !data.is_empty() {
                data.push('.');
            }
            data.push_str(suffix);
        }

        Self {
            data: data.into(),
            is_gate: false,
        }
    }

    /// Retruns a new object path pointing to the gate on the current module.
    #[must_use]
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
        let mut this = Self {
            data: s.into(),
            is_gate: false,
        };
        if this.name().contains('#') {
            this.is_gate = true;
        }
        this
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

impl PartialEq<&str> for ObjectPath {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialOrd for ObjectPath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ObjectPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.cmp(&other.data)
    }
}

impl Default for ObjectPath {
    fn default() -> Self {
        Self {
            data: String::new().into(),
            is_gate: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_appending() {
        let path = ObjectPath::default().appended("top").appended("mid");

        assert_eq!(path.name(), "mid");
        assert_eq!(path.as_parent_str(), "top");
        assert_eq!(
            path,
            ObjectPath {
                data: "top.mid".to_string().into(),
                is_gate: false,
            }
        );

        let path = ObjectPath::default()
            .appended("top")
            .appended("mid")
            .appended("low");

        assert_eq!(path.name(), "low");
        assert_eq!(path.as_parent_str(), "top.mid");
        assert_eq!(
            path,
            ObjectPath {
                data: "top.mid.low".to_string().into(),
                is_gate: false,
            }
        );

        let path = ObjectPath::default().appended("top");
        assert_eq!(path.name(), "top");
        assert_eq!(path.as_parent_str(), "");
        assert_eq!(
            path,
            ObjectPath {
                data: "top".to_string().into(),
                is_gate: false,
            }
        );

        let path = ObjectPath::default();
        assert_eq!(path.name(), "");
        assert_eq!(path.as_parent_str(), "");
        assert!(path.is_root());
        assert_eq!(
            path,
            ObjectPath {
                data: String::new().into(),
                is_gate: false,
            }
        );
    }

    #[test]
    fn parent_creation() {
        let path = ObjectPath::default().appended("top").appended("mid");
        let parent = path.parent();
        assert_eq!(
            parent,
            Some(ObjectPath {
                data: "top".to_string().into(),
                is_gate: false,
            })
        );

        let path = ObjectPath::default()
            .appended("top")
            .appended("mid")
            .appended("low");

        let parent = path.parent();
        assert_eq!(
            parent,
            Some(ObjectPath {
                data: "top.mid".to_string().into(),
                is_gate: false,
            })
        );

        let path = ObjectPath::default();
        let parent = path.parent();
        assert_eq!(parent, None);
    }

    #[test]
    fn parsing() {
        assert_eq!(
            ObjectPath::from("top.mid"),
            ObjectPath {
                data: "top.mid".to_string().into(),
                is_gate: false,
            }
        );

        assert_eq!(
            ObjectPath::from("top.mid.low"),
            ObjectPath {
                data: "top.mid.low".to_string().into(),
                is_gate: false,
            }
        );

        assert_eq!(
            ObjectPath::from("top"),
            ObjectPath {
                data: "top".to_string().into(),
                is_gate: false,
            }
        );

        assert_eq!(
            ObjectPath::from(""),
            ObjectPath {
                data: String::new().into(),
                is_gate: false,
            }
        );

        // emoji is a 4 byte character thus 7 + 4
        assert_eq!(
            ObjectPath::from("top.a😀b.low"),
            ObjectPath {
                data: "top.a😀b.low".to_string().into(),
                is_gate: false,
            }
        );

        assert!(ObjectPath::default().is_root());
        assert!(ObjectPath::from(String::new()).is_module());
        assert_eq!(ObjectPath::from(&String::new()).as_logger_scope(), "@root");
        assert_eq!(ObjectPath::from("abc").as_logger_scope(), "abc");
        assert!(!ObjectPath::from("a").appended_gate("gate").is_module());
        assert!(ObjectPath::from("root.a.b.c").as_ref().starts_with("root"));
    }
}
