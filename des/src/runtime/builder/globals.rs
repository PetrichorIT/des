use std::{path::PathBuf, sync::Mutex};

use crate::{
    module::{Cfg, ModuleRef, Props},
    statistics::Statistics,
};

///
/// The global parameters about a [`Sim`] that are publicly
/// exposed.
///
#[derive(Debug, Default)]
pub struct Globals {
    pub(crate) roots: Mutex<ModuleRoots>,
    pub(crate) cfgs: Mutex<Vec<Cfg>>,
    pub(crate) dir: Mutex<PathBuf>,
    pub(crate) statistics: Mutex<Statistics>,
}

impl Globals {
    pub(crate) fn with<R>(&self, f: impl FnOnce(&ModuleRoots) -> R) -> R {
        f(&self.roots.lock().expect("failed"))
    }

    /// Returns a handle to a module from the global scope.
    /// This can be used to access arbitrary modules, independent of the current execution context.
    #[must_use]
    pub fn get(&self, path: impl AsRef<str>) -> Option<ModuleRef> {
        self.with(|mods| mods.get(path.as_ref()))
    }

    /// Returns the directory path of the
    /// out directory for this simulation.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn dir(&self) -> PathBuf {
        self.dir.lock().expect("failed").clone()
    }

    pub(crate) fn add_module(&self, module: ModuleRef) {
        self.roots.lock().expect("failed").add(module);
    }

    pub(crate) fn add_cfg(&self, cfg: Cfg) {
        self.cfgs.lock().expect("failed").push(cfg);
    }

    pub(crate) fn capture_for(&self, path_parts: &[&str], props: &mut Props) {
        let lock = self.cfgs.lock().expect("failed");
        for cfg in &*lock {
            cfg.capture_for(path_parts, props);
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ModuleRoots {
    modules: Vec<ModuleRef>,
}

/// The all nodes iterator.
struct AllNodesIter<'a> {
    stack: Vec<(ModuleRef, Vec<String>)>,
    remaining: &'a [ModuleRef],
}

impl Iterator for AllNodesIter<'_> {
    type Item = ModuleRef;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_along_stack().or_else(|| {
            assert!(self.stack.is_empty());
            let next_root = self.remaining.first()?.clone();
            self.remaining = &self.remaining[1..];
            self.stack.push((
                next_root.clone(),
                next_root.children.read().keys().cloned().collect(),
            ));
            Some(next_root)
        })
    }
}

impl AllNodesIter<'_> {
    fn next_along_stack(&mut self) -> Option<ModuleRef> {
        let (node, keys) = self.stack.last_mut()?;
        let Some(key) = keys.pop() else {
            self.stack.pop();
            return self.next_along_stack();
        };

        let child = node.children.read()[&key].clone();
        self.stack.push((
            child.clone(),
            child.children.read().keys().cloned().collect(),
        ));
        Some(child)
    }
}

impl ModuleRoots {
    pub(crate) fn nodes(&self) -> impl Iterator<Item = ModuleRef> + '_ {
        AllNodesIter {
            stack: Vec::new(),
            remaining: &self.modules,
        }
    }

    pub(crate) fn get(&self, path: &str) -> Option<ModuleRef> {
        let (first, mut rem) = if self.modules.first()?.path.is_root() {
            ("", path)
        } else {
            path.split_once('.').unwrap_or((path, ""))
        };
        let mut current = self.modules.iter().find(|m| m.path == first)?.clone();

        while !rem.is_empty() {
            let (next, rest) = rem.split_once('.').unwrap_or((rem, ""));
            rem = rest;
            current = current.child(next).ok()?;
        }

        Some(current)
    }

    pub(crate) fn add(&mut self, module: ModuleRef) {
        assert!(
            module.parent.is_none(), // && dbg!(module.path.parent()).is_none(),
            "cannot register non-root module as root"
        );
        match self
            .modules
            .binary_search_by_key(&&module.path, |m| &m.path)
        {
            Ok(i) | Err(i) => self.modules.insert(i, module),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Weak;

    use super::*;
    use crate::module::ModuleContext;

    #[test]
    fn module_tree() {
        let mut tree = ModuleRoots::default();
        fn module(path: &str) -> ModuleRef {
            ModuleContext::new_root(path.into(), Weak::new())
        }

        tree.add(module("alice"));
        tree.add(module("bob"));
        tree.add(module("eve"));

        assert_eq!(
            tree.nodes().map(|v| v.path.to_string()).collect::<Vec<_>>(),
            ["alice", "bob", "eve",]
        );
    }
}
