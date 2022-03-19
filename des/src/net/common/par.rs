use std::collections::HashMap;

///
/// The collection of all loaded parameters for modules,
/// inside a network runtime.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameters {
    tree: ParameterTree,
}

impl Parameters {
    pub fn new() -> Self {
        Self {
            tree: ParameterTree::new(),
        }
    }

    pub fn build(&mut self, raw_text: &str) {
        for line in raw_text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                self.insert(key.trim(), value.trim());
            }
        }
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.tree.insert(key, value)
    }

    pub(crate) fn get(&self, key: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        self.tree.get(key, &mut map);
        map
    }
}

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
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
}
