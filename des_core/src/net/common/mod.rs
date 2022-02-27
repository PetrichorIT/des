use std::fmt::Display;

///
/// A unqiue identifier for a module, indicating its parental inheritance
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath {
    full_path: String,
    name_start: usize,
}

impl ModulePath {
    pub fn module_name(&self) -> &str {
        &self.full_path[self.name_start..]
    }

    pub fn module_path(&self) -> &str {
        &self.full_path
    }

    pub fn parent_path(&self) -> &str {
        if self.name_start == 0 {
            &self.full_path[..0]
        } else {
            &self.full_path[..self.name_start - 1]
        }
    }

    pub fn root(name: String) -> Self {
        Self {
            full_path: name,
            name_start: 0,
        }
    }

    pub fn new_with_parent(name: &str, parent: &ModulePath) -> Self {
        let full_path = format!("{}.{}", parent.full_path, name);
        let name_start = parent.full_path.len() + 1;

        Self {
            full_path,
            name_start,
        }
    }
}

impl Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_path)
    }
}
