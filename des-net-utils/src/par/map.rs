use std::{
    fmt::Display,
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
};

use fxhash::FxHashMap;

use super::yaml::yaml_to_par_map;

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
    /// Creates new entries from a raw input text.
    ///
    /// See [`Sim::include_par`](crate::net::Sim) for more infomation.
    pub fn build(&self, raw_text: &str) {
        let Ok(map) = yaml_to_par_map(raw_text) else {
            return;
        };
        for (key, value) in map {
            self.insert(key.trim(), value.trim().to_string());
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let val = self.get_rlock(key, 1)?;
        self.release_rlock(key);
        Some(val)
    }

    pub fn get_rlock(&self, key: &str, inc: usize) -> Option<String> {
        self.tree.read().unwrap().get_rlock(key, inc)
    }

    pub fn release_rlock(&self, key: &str) {
        let done = self.tree.read().unwrap().release_rlock(key);
        assert!(done);
    }

    pub fn keys(&self, module: &str) -> Vec<String> {
        let mut keys = Vec::new();
        self.tree
            .write()
            .expect("failed to get lock")
            .keys(module, &mut keys);
        keys
    }

    pub fn insert(&self, key: &str, value: String) -> bool {
        self.tree.write().unwrap().insert(key, value)
    }

    pub fn remove(&self, key: &str) {
        self.tree.write().unwrap().remove(key);
    }

    pub fn export(&self, writer: &mut impl io::Write) -> io::Result<()> {
        self.tree.read().unwrap().export(writer, "")
    }
}

impl ParTree {
    fn new() -> ParTree {
        ParTree {
            branches: Vec::new(),
            pars: FxHashMap::default(),
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
                    lock.fetch_add(inc, Ordering::SeqCst);
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
                    lock.fetch_sub(1, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn keys(&self, key: &str, result: &mut Vec<String>) {
        if let Some((comp, remainder)) = key.split_once('.') {
            for branch in self.branches.iter().filter(|b| b.matching.matches_r(comp)) {
                branch.node.keys(remainder, result);
            }
        } else {
            // last recursion
            for branch in self.branches.iter().filter(|b| b.matching.matches_r(key)) {
                result.extend(branch.node.pars.keys().cloned());
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
                if comp == "*" || comp == "_" {
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
            if entry.1.load(Ordering::SeqCst) == 0 {
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
            Self::Any => key == "*" || key == "_",
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
            Self::Any => write!(f, "_"),
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
