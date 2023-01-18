use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Mutex, RwLock};

use crate::net::globals;

///
/// The collection of all loaded parameters for modules,
/// inside a network runtime.
///
#[derive(Debug)]
pub struct Parameters {
    tree: RwLock<ParameterTree>,
    pub(crate) updates: Mutex<Vec<String>>,
}

impl Parameters {
    ///
    /// Creates a new empty parameter tree.
    ///
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: RwLock::new(ParameterTree::new()),
            updates: Mutex::new(Vec::new()),
        }
    }

    ///
    /// Populates the parameter tree using the given raw text
    /// as parameter definitions.
    ///
    pub fn build(&self, raw_text: &str) {
        for line in raw_text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                self.insert(key.trim(), value.trim());
            }
        }
    }

    pub(crate) fn insert(&self, key: &str, value: &str) {
        self.tree.write().unwrap().insert(key, value);
    }

    pub(crate) fn get_def_table(&self, key: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        self.tree.read().unwrap().get(key, &mut map);
        map
    }

    ///
    /// Creates a read-and-write handle to a specific key on a module.
    ///
    /// This handle can point to a nonexiting value if its only used for writing.
    ///
    #[must_use]
    pub fn get_handle(&self, path: &str, key: &str) -> ParHandle<Optional> {
        ParHandle {
            path: path.to_string(),
            key: key.to_string(),

            value: None,

            _phantom: PhantomData,
        }
    }
}

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

mod private {
    pub trait ParHandleState {}
}

///
/// The state of a [`ParHandle`] where its not decided
/// whether data is contained or not. Useful for writing data
/// to not yet initalized parameters.
///
#[derive(Debug)]
pub struct Optional;
impl private::ParHandleState for Optional {}

/// The state of a [`ParHandle`] where the contents are guaranteed
/// to be there, thus allowing derefs on the handle.
///
#[derive(Debug)]
pub struct Unwraped;
impl private::ParHandleState for Unwraped {}

///
/// A handle for a requested parameter, local to a
/// module path and parameter key.
///
#[derive(Debug)]
pub struct ParHandle<State>
where
    State: private::ParHandleState,
{
    path: String,
    key: String,

    value: Option<String>,
    _phantom: PhantomData<State>,
}

impl<State> ParHandle<State>
where
    State: private::ParHandleState,
{
    ///
    /// Unwraps the handle allowing [Deref] on the contained
    /// value consuming self.
    ///
    /// # Panics
    ///
    /// Panics if the handle points to no existing value.
    ///
    #[must_use]
    pub fn unwrap(self) -> ParHandle<Unwraped> {
        if let Some(val) = globals()
            .parameters
            .tree
            .read()
            .unwrap()
            .get_value(&self.path, &self.key)
        {
            ParHandle {
                path: self.path,
                key: self.key,

                value: Some(val.to_string()),

                _phantom: PhantomData,
            }
        } else {
            panic!(
                "Unwraped par handle that did not point to data: {} / {}",
                self.path, self.key
            )
        }
    }

    ///
    /// Maps the internal value if exisitent to a given output.
    ///
    pub fn map<F, T>(self, mut f: F) -> Option<T>
    where
        F: FnMut(ParHandle<Unwraped>) -> T,
    {
        if self.is_some() {
            Some(f(self.unwrap()))
        } else {
            None
        }
    }

    ///
    /// Indicates whether the handle contains a value.
    ///
    #[must_use]
    pub fn is_some(&self) -> bool {
        self.value.is_some()
            || globals()
                .parameters
                .tree
                .read()
                .unwrap()
                .get_value(&self.path, &self.key)
                .is_some()
    }

    ///
    /// Indicates whether the handle contains a value.
    ///
    #[must_use]
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    ///
    /// Returns the contained value optionally, thereby losing the
    /// ability to set the par.
    ///
    #[must_use]
    pub fn as_optional(self) -> Option<String> {
        match self.value {
            Some(value) => Some(value),
            None => globals()
                .parameters
                .tree
                .read()
                .unwrap()
                .get_value(&self.path, &self.key)
                .map(str::to_string),
        }
    }

    ///
    /// Sets the parameter to the given value.
    ///
    #[allow(clippy::needless_pass_by_value)]
    pub fn set<T>(self, value: T)
    where
        T: ToString,
    {
        let str = value.to_string();
        globals()
            .parameters
            .tree
            .write()
            .unwrap()
            .insert(&format!("{}.{}", self.path, self.key), &str);

        globals().parameters.updates.lock().unwrap().push(self.path);
    }
}

impl ParHandle<Unwraped> {
    ///
    /// Uses a custom string parser to parse a string, timming
    /// quotation marks in the process.
    ///
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn parse_string(&self) -> String {
        let mut parsed = self.value.clone().unwrap();
        // Trim marks
        let mut chars = parsed.chars();
        let mut is_marked = parsed.len() >= 2;
        is_marked &= chars.next() == Some('"');
        is_marked &= chars.next_back() == Some('"');

        if is_marked {
            parsed.pop();
            parsed.remove(0);
            parsed
        } else {
            parsed
        }
    }
}

impl Deref for ParHandle<Unwraped> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.value.as_ref().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParameterTreeBranch {
    Path(String, ParameterTree),
    Asterix(ParameterTree),
}

impl ParameterTreeBranch {
    fn matches(&self, key: &str) -> bool {
        match self {
            Self::Path(path, ..) => path == key,
            Self::Asterix(..) => key == "*",
        }
    }

    fn tree_mut(&mut self) -> &mut ParameterTree {
        match self {
            Self::Asterix(tree) | Self::Path(_, tree) => tree,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParameterTree {
    branches: Vec<ParameterTreeBranch>,
    pars: HashMap<String, String>,
}

impl ParameterTree {
    fn new() -> Self {
        Self {
            branches: Vec::new(),
            pars: HashMap::new(),
        }
    }

    fn insert(&mut self, key: &str, value: &str) {
        match key.split_once('.') {
            Some((ele, rem)) => {
                if let Some(branch) = self.branches.iter_mut().find(|b| b.matches(ele)) {
                    branch.tree_mut().insert(rem, value);
                } else {
                    let mut node = ParameterTree::new();
                    node.insert(rem, value);
                    if ele == "*" {
                        self.branches.push(ParameterTreeBranch::Asterix(node));
                    } else {
                        self.branches
                            .push(ParameterTreeBranch::Path(ele.to_string(), node));
                    }
                }
            }
            None => {
                self.pars.insert(key.to_string(), value.to_string());
            }
        }
    }

    fn get(&self, key: &str, map: &mut HashMap<String, String>) {
        if key.is_empty() {
            self.pars.iter().for_each(|(key, value)| {
                map.insert(key.to_string(), value.to_string());
            });
        }
        let (ele, rem) = key.split_once('.').unwrap_or((key, ""));

        for branch in &self.branches {
            match branch {
                ParameterTreeBranch::Asterix(subtree) => subtree.get(rem, map),
                ParameterTreeBranch::Path(path, subtree) => {
                    if path == ele {
                        subtree.get(rem, map);
                    }
                }
            }
        }
    }

    fn get_value(&self, path: &str, key: &str) -> Option<&str> {
        if path.is_empty() {
            // Found final node.
            self.pars.get(key).map(|s| &s[..])
        } else {
            let (ele, rem) = path.split_once('.').unwrap_or((path, ""));
            // Go via exact branch if possible;
            let ret_val = self.branches.iter().find_map(|b| {
                if let ParameterTreeBranch::Path(path, subtree) = b {
                    if path == ele {
                        return subtree.get_value(rem, key);
                    }
                }
                None
            });

            if ret_val.is_some() {
                return ret_val;
            }

            // Asterix search
            self.branches.iter().find_map(|b| {
                if let ParameterTreeBranch::Asterix(subtree) = b {
                    return subtree.get_value(rem, key);
                }
                None
            })
        }
    }
}
