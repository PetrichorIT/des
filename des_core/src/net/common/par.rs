use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameters {
    tree: ParameterTree,
}

impl Parameters {
    pub fn new() -> Self {
        Self {
            tree: ParameterTree::Node {
                branches: Vec::new(),
            },
        }
    }

    pub fn build(&mut self, raw_text: &str) {
        for line in raw_text.lines() {
            if let Some((key, value)) = line.split_once("=") {
                self.insert(key.trim(), value.trim());
            }
        }
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.tree.insert(key, value)
    }

    pub fn get(&self, key: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        self.tree.get(key, &mut map);
        map
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
enum ParameterTree {
    Node { branches: Vec<ParameterTreeBranch> },
    Leaf { pars: HashMap<String, String> },
}

impl ParameterTree {
    fn insert(&mut self, key: &str, value: &str) {
        match self {
            Self::Node { branches } => {
                // Search or create matching branch
                match key.split_once(".") {
                    Some((ele, rem)) => match branches.iter_mut().find(|b| b.matches(ele)) {
                        Some(branch) => branch.tree_mut().insert(rem, value),
                        None => {
                            let mut node = ParameterTree::Node {
                                branches: Vec::new(),
                            };
                            node.insert(rem, value);
                            if ele == "*" {
                                branches.push(ParameterTreeBranch::Asterix(node))
                            } else {
                                branches.push(ParameterTreeBranch::Path(ele.to_string(), node))
                            }
                        }
                    },
                    None => {
                        // ASsumming this is a end point, and a recently created node
                        assert!(branches.is_empty());

                        let mut map = HashMap::new();
                        map.insert(key.to_string(), value.to_string());

                        *self = ParameterTree::Leaf { pars: map }
                    }
                }
            }
            Self::Leaf { pars } => {
                // Assume that key is now only the identifier for the parameter name
                assert!(!key.contains('.'));
                pars.insert(key.to_string(), value.to_string());
            }
        }
    }

    fn get(&self, key: &str, map: &mut HashMap<String, String>) {
        match self {
            Self::Node { branches } => {
                let (ele, rem) = key.split_once(".").unwrap_or((key, ""));
                for branch in branches {
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
            Self::Leaf { pars } => {
                if key == "" {
                    pars.iter().for_each(|(key, value)| {
                        let _ = map.insert(key.to_string(), value.to_string());
                    })
                }
            }
        }
    }
}
