use fxhash::{FxBuildHasher, FxHashMap};
use std::fmt::Display;
use std::io;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, RwLock};

mod api;
mod yaml;

pub use self::api::*;
use super::globals;
use crate::net::par::yaml::yaml_to_par_map;

// # Internal mappings

/// A storage for all parameters associated with a simulation.
#[derive(Debug)]
pub struct ParMap {
    tree: RwLock<ParTree>,
}

#[derive(Debug)]
struct ParTree {
    branches: Vec<ParTreeBranch>,
    pars: FxHashMap<String, (String, AtomicUsize)>,
}

#[derive(Debug)]
struct ParTreeBranch {
    matching: ParTreePathMatching,
    node: ParTree,
}

#[derive(Debug)]
enum ParTreePathMatching {
    Any,
    Path(String),
}

impl ParMap {
    fn shared() -> Arc<ParMap> {
        globals().parameters.clone()
    }

    /// Creates new entries from a raw input text.
    ///
    /// See [`Sim::include_par`](crate::net::Sim) for more infomation.
    pub fn build(&self, raw_text: &str) {
        let map = yaml_to_par_map(raw_text).expect("failed to parse par");
        for (key, value) in map {
            self.insert(key.trim(), value.trim().to_string());
        }
    }

    fn get_rlock(&self, key: &str, inc: usize) -> Option<String> {
        self.tree.read().unwrap().get_rlock(key, inc)
    }

    fn release_rlock(&self, key: &str) {
        let done = self.tree.read().unwrap().release_rlock(key);
        assert!(done);
    }

    fn insert(&self, key: &str, value: String) -> bool {
        self.tree.write().unwrap().insert(key, value)
    }

    fn remove(&self, key: &str) {
        self.tree.write().unwrap().remove(key);
    }

    fn export(&self, writer: &mut impl io::Write) -> io::Result<()> {
        self.tree.read().unwrap().export(writer, "")
    }
}

impl ParTree {
    fn new() -> ParTree {
        ParTree {
            branches: Vec::new(),
            pars: FxHashMap::with_hasher(FxBuildHasher::default()),
        }
    }

    fn get_rlock(&self, key: &str, inc: usize) -> Option<String> {
        match key.split_once('.') {
            Some((comp, remainder)) => {
                for branch in self.branches.iter().filter(|b| b.matching.matches_r(comp)) {
                    let Some(ret) = branch.node.get_rlock(remainder, inc) else {
                        continue;
                    };
                    return Some(ret);
                }
                None
            }
            None => {
                if let Some((value, lock)) = self.pars.get(key) {
                    lock.fetch_add(inc, SeqCst);
                    Some(value.clone())
                } else {
                    None
                }
            }
        }
    }

    fn release_rlock(&self, key: &str) -> bool {
        match key.split_once('.') {
            Some((comp, rem)) => {
                for branch in self.branches.iter().filter(|b| b.matching.matches_r(comp)) {
                    if branch.node.release_rlock(rem) {
                        return true;
                    }
                }
                false
            }
            None => {
                if let Some((_, lock)) = self.pars.get(key) {
                    lock.fetch_sub(1, SeqCst);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn insert(&mut self, key: &str, value: String) -> bool {
        if let Some((comp, remainder)) = key.split_once('.') {
            if let Some(branch) = self
                .branches
                .iter_mut()
                .find(|b| b.matching.matches_w(comp))
            {
                branch.node.insert(remainder, value)
            } else {
                let mut node = ParTree::new();
                let ret = node.insert(remainder, value);
                if comp == "*" {
                    self.branches.push(ParTreeBranch {
                        matching: ParTreePathMatching::Any,
                        node,
                    });
                } else {
                    self.branches.push(ParTreeBranch {
                        matching: ParTreePathMatching::Path(comp.to_string()),
                        node,
                    });
                }
                ret
            }
        } else {
            // (0) Fetch the entry
            let entry = self
                .pars
                .entry(key.to_string())
                .or_insert((String::new(), AtomicUsize::new(0)));

            // (1) try an inplace update (requires not readers)
            if entry.1.load(SeqCst) == 0 {
                entry.0 = value;
                true
            } else {
                false
            }
        }
    }

    fn remove(&mut self, key: &str) -> bool {
        match key.split_once('.') {
            Some((comp, rem)) => self
                .branches
                .iter_mut()
                .find(|b| b.matching.matches_w(comp))
                .is_some_and(|b| b.node.remove(rem)),
            None => self.pars.remove(key).is_some(),
        }
    }

    fn export(&self, writer: &mut impl io::Write, path: &str) -> io::Result<()> {
        // Write pars directly
        for (key, (value, _)) in &self.pars {
            writeln!(writer, "{path}.{key}: {value}")?;
        }

        // Recurse branches
        for branch in &self.branches {
            let new_path = if path.is_empty() {
                branch.matching.to_string()
            } else {
                format!("{path}.{}", branch.matching)
            };
            branch.node.export(writer, &new_path)?;
        }

        Ok(())
    }
}

impl ParTreePathMatching {
    fn matches_w(&self, key: &str) -> bool {
        match self {
            Self::Any => key == "*",
            Self::Path(ref path) => path == key,
        }
    }

    fn matches_r(&self, key: &str) -> bool {
        // dbg!(self, key);
        match self {
            Self::Any => true,
            Self::Path(ref path) => path == key,
        }
    }
}

impl Display for ParTreePathMatching {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Any => write!(f, "*"),
            Self::Path(path) => write!(f, "{path}"),
        }
    }
}

impl Default for ParMap {
    fn default() -> Self {
        ParMap {
            tree: RwLock::new(ParTree::new()),
        }
    }
}

// # External API

/// A handle to a parameter associated to a node within the simulation.
///
/// This type is parameterized by a type-state parameter `S`.
/// This parameter indicates whether the parameter is guaranteed to
/// exist `S = Exists` or this remains in question `S = Optional`.
///
/// This type provides methods to read an write parameters, based on the
/// type state. `Par<Exists>` implement `Deref<Target = str>` so parameters
/// can be extracted and perhaps parsed, as soon as the existence of the parameter
/// is confirmed.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Par<S = Optional>
where
    S: private::ParState,
{
    key: String,
    value: Option<String>,

    _phantom: PhantomData<S>,
}

/// The state of a [`Par`] where its not decided
/// whether data is contained or not. Useful for writing data
/// to not yet initalized parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Optional;
impl private::ParState for Optional {}

/// The state of a [`Par`] where the contents are guaranteed
/// to be there, thus allowing derefs on the handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Exists;
impl private::ParState for Exists {}

/// Errors that can occur in combination with [`Par`] objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParError {
    /// This error occures if a write operation failed, since a write lock could
    /// not be aquired.
    CouldNotAquireWriteLock,
}

mod private {
    pub trait ParState {}
}

impl Par<Optional> {
    fn new(key: &str, module: &str) -> Par {
        if module.is_empty() {
            Par {
                key: key.to_string(),
                value: None,
                _phantom: PhantomData,
            }
        } else {
            Par {
                key: format!("{module}.{key}"),
                value: None,
                _phantom: PhantomData,
            }
        }
    }
}

impl<S> Par<S>
where
    S: private::ParState,
{
    /// Returns a handle allowing [`Deref`] on the contained
    /// value, consuming self.
    ///
    /// # Examples
    ///
    /// This example would succeed:
    /// ```
    /// # use des::prelude::*;
    /// # use des::net::*;
    /// let mut sim = Sim::new(());
    /// sim.node("alice", ModuleFn::new(
    ///     || {
    ///         let par = par("addr")
    ///             .expect("parameter 'addr' does not exist")
    ///             .parse::<IpAddr>()
    ///             .expect("parameter 'addr' failed to be parsed");
    ///     },
    ///     |_, _| {}
    /// ));
    /// sim.include_par("alice.addr: 198.168.2.1\n");
    /// /* ... */
    ///
    /// let _ = Builder::new().build(sim).run();
    /// ```
    ///
    /// While this would panic:
    ///
    /// ```should_panic
    /// # use des::prelude::*;
    /// # use des::net::*;
    /// let mut sim = Sim::new(());
    /// sim.node("alice", ModuleFn::new(
    ///     || {
    ///         let par = par("addr")
    ///             .expect("parameter 'addr' does not exist")
    ///             .parse::<IpAddr>()
    ///             .expect("parameter 'addr' failed to be parsed");
    ///     },
    ///     |_, _| {}
    /// ));
    ///
    /// let _ = Builder::new().build(sim).run();
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics of the Par points to no data.
    ///
    #[must_use]
    pub fn expect(self, msg: &str) -> Par<Exists> {
        let map = ParMap::shared();
        if let Some(value) = map.get_rlock(&self.key, 1) {
            Par {
                key: self.key.clone(),
                value: Some(value),
                _phantom: PhantomData,
            }
        } else {
            panic!("{msg}");
        }
    }

    /// Returns a handle allowing [`Deref`] on the contained
    /// value, consuming self.
    ///
    /// See [`Par::expect`] for more information.
    #[must_use]
    pub fn unwrap(self) -> Par<Exists> {
        self.expect("called `Par::unwrap` on a parameter that does not exist")
    }

    /// Indicates whether the handle contains a value.
    #[must_use]
    pub fn is_some(&self) -> bool {
        // (0) Shortciruit
        if self.value.is_some() {
            return true;
        }

        // (1) Long way around
        let map = ParMap::shared();
        map.get_rlock(&self.key, 0).is_some()
    }

    /// Indicates whether the handle contains a value.
    #[must_use]
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    /// Returns the contained value optionaly, thereby losing the
    /// ability to set the par. This does not create a permantent
    /// read lock.
    #[must_use]
    pub fn as_option(self) -> Option<String> {
        let map = ParMap::shared();
        map.get_rlock(&self.key, 0)
    }

    /// Sets the parameter to the given value.
    ///
    /// # Errors
    ///
    /// Returns an error if other active locks exist for the datapoint.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::net::*;
    /// let mut sim = Sim::new(());
    /// sim.node("alice", ModuleFn::new(
    ///     || {
    ///         assert!(par("addr").is_none());
    ///         par("addr").set(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 110)));
    ///         assert!(par("addr").is_some());
    ///         assert_eq!(&*par("addr").unwrap(), "192.168.2.110");
    ///     },
    ///     |_, _| {}
    /// ));
    ///
    /// let _ = Builder::new().build(sim).run();
    /// ```
    #[allow(clippy::needless_pass_by_value)]
    pub fn set(self, value: impl ToString) -> Result<Par<Exists>, ParError> {
        let map = ParMap::shared();
        let value = value.to_string();
        if map.insert(&self.key, value) {
            Ok(Par {
                key: self.key.clone(),
                value: map.get_rlock(&self.key, 1),
                _phantom: PhantomData,
            })
        } else {
            Err(ParError::CouldNotAquireWriteLock)
        }
    }

    /// Remove the entry from the par storage.
    ///
    /// Returns a `Par` object with optional (in this case None) content.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::net::*;
    /// let mut sim = Sim::new(());
    /// sim.node("alice", ModuleFn::new(
    ///     || {
    ///         assert!(par("addr").is_some());
    ///         par("addr").unset();
    ///         assert!(par("addr").is_none());
    ///     },
    ///     |_, _| {}
    /// ));
    /// sim.include_par("alice.addr: 192.168.2.110");
    ///
    /// let _ = Builder::new().build(sim).run();
    /// ```
    #[must_use]
    pub fn unset(self) -> Par<Optional> {
        let map = ParMap::shared();
        map.remove(&self.key);
        Par {
            value: None,
            key: self.key.clone(),
            _phantom: PhantomData,
        }
    }
}

impl Par<Exists> {
    /// Uses a custom string parser to parse a string, timming
    /// quotation marks in the process.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn into_inner(&self) -> String {
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

impl Deref for Par<Exists> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.value.as_ref().unwrap()
    }
}

impl<S> Clone for Par<S>
where
    S: private::ParState,
{
    fn clone(&self) -> Self {
        if self.value.is_some() {
            ParMap::shared().get_rlock(&self.key, 1);
        }

        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S> Drop for Par<S>
where
    S: private::ParState,
{
    fn drop(&mut self) {
        // (0) Only if Par<Exists>
        if self.value.is_some() {
            let map = ParMap::shared();
            map.release_rlock(&self.key);
        }
    }
}
