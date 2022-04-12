use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;

///
/// The collection of all loaded parameters for modules,
/// inside a network runtime.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameters {
    tree: RefCell<ParameterTree>,
}

impl Parameters {
    ///
    /// Creates a new empty parameter tree.
    ///
    pub fn new() -> Self {
        Self {
            tree: RefCell::new(ParameterTree::new()),
        }
    }

    ///
    /// Populates the parameter tree using the given raw text
    /// as parameter definitions.
    ///
    pub fn build(&mut self, raw_text: &str) {
        for line in raw_text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                self.insert(key.trim(), value.trim());
            }
        }
    }

    pub(crate) fn insert(&mut self, key: &str, value: &str) {
        self.tree.borrow_mut().insert(key, value)
    }

    pub(crate) fn get(&self, key: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        self.tree.borrow().get(key, &mut map);
        map
    }

    pub(crate) fn get_handle<'a>(&'a self, path: &'a str, key: &'a str) -> ParHandle<'a, Optional> {
        ParHandle {
            tree_ref: &self.tree,
            path,
            key,

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
/// The state of a [ParHandle] where its not decided
/// whether data is contained or not. Useful for writing data
/// to not yet initalized parameters.
///
pub struct Optional;
impl private::ParHandleState for Optional {}

/// The state of a [ParHandle] where the contents are guaranteed
/// to be there, thus allowing derefs on the handle.
///
pub struct Unwraped;
impl private::ParHandleState for Unwraped {}

///
/// A handle for a requested parameter, local to a
/// module path and parameter key.
///
#[derive(Debug)]
pub struct ParHandle<'a, State>
where
    State: private::ParHandleState,
{
    tree_ref: &'a RefCell<ParameterTree>,
    path: &'a str,
    key: &'a str,

    value: Option<String>,

    _phantom: PhantomData<State>,
}

impl<'a, State> ParHandle<'a, State>
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
    pub fn unwrap(self) -> ParHandle<'a, Unwraped> {
        if let Some(val) = self.tree_ref.borrow().get_value(self.path, self.key) {
            ParHandle {
                tree_ref: self.tree_ref,
                path: self.path,
                key: self.key,

                value: Some(val.to_string()),

                _phantom: PhantomData,
            }
        } else {
            panic!("Unwraped par handle that did point to data")
        }
    }

    ///
    /// Maps the internal value if exisitent to a given output.
    ///
    pub fn map<F, T>(self, mut f: F) -> Option<T>
    where
        F: FnMut(ParHandle<'_, Unwraped>) -> T,
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
    pub fn is_some(&self) -> bool {
        self.value.is_some()
            || self
                .tree_ref
                .borrow()
                .get_value(self.path, self.key)
                .is_some()
    }

    ///
    /// Indicates whether the handle contains a value.
    ///
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    ///
    /// Returns the contained value optionally, thereby losing the
    /// ability to set the par.
    ///
    pub fn as_optional(self) -> Option<String> {
        match self.value {
            Some(value) => Some(value),
            None => self
                .tree_ref
                .borrow()
                .get_value(self.path, self.key)
                .map(str::to_string),
        }
    }

    ///
    /// Sets the parameter to the given value.
    ///
    pub fn set<T>(self, value: T)
    where
        T: ToString,
    {
        let str = value.to_string();
        self.tree_ref
            .borrow_mut()
            .insert(&format!("{}.{}", self.path, self.key), &str);
        // (*self.tree_ref).insert(&self.path_and_key, &str);
    }
}

impl Deref for ParHandle<'_, Unwraped> {
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
            Self::Path(_, tree) => tree,
            Self::Asterix(tree) => tree,
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
            Some((ele, rem)) => match self.branches.iter_mut().find(|b| b.matches(ele)) {
                Some(branch) => branch.tree_mut().insert(rem, value),
                None => {
                    let mut node = ParameterTree::new();
                    node.insert(rem, value);
                    if ele == "*" {
                        self.branches.push(ParameterTreeBranch::Asterix(node))
                    } else {
                        self.branches
                            .push(ParameterTreeBranch::Path(ele.to_string(), node))
                    }
                }
            },
            None => {
                self.pars.insert(key.to_string(), value.to_string());
            }
        }
    }

    fn get(&self, key: &str, map: &mut HashMap<String, String>) {
        if key.is_empty() {
            self.pars.iter().for_each(|(key, value)| {
                let _ = map.insert(key.to_string(), value.to_string());
            })
        }
        let (ele, rem) = key.split_once('.').unwrap_or((key, ""));

        for branch in &self.branches {
            match branch {
                ParameterTreeBranch::Asterix(subtree) => subtree.get(rem, map),
                ParameterTreeBranch::Path(path, subtree) => {
                    if path == ele {
                        subtree.get(rem, map)
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
            let ret = self.branches.iter().find_map(|b| {
                if let ParameterTreeBranch::Path(path, subtree) = b {
                    if path == ele {
                        return subtree.get_value(rem, key);
                    }
                }
                None
            });

            if ret.is_some() {
                return ret;
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
