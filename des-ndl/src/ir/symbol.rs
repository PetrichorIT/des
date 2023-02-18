use std::sync::Arc;

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct RawSymbol {
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    Module(Arc<Module>),
    Link(Arc<Link>),
    Unresolved(String),
}

// # Impl

impl RawSymbol {
    pub fn could_be_submodule(&self, ident: impl AsRef<str>, cluster: &Cluster) -> bool {
        self.raw == format!("{}{}", ident.as_ref(), cluster)
    }
}

impl Symbol {
    pub fn as_link(&self) -> Option<&Link> {
        match self {
            Self::Link(l) => Some(&**l),
            _ => None,
        }
    }

    pub fn as_module(&self) -> Option<&Module> {
        match self {
            Self::Module(m) => Some(&**m),
            _ => None,
        }
    }

    pub fn as_link_arc(&self) -> Option<Arc<Link>> {
        match self {
            Self::Link(l) => Some(l.clone()),
            _ => None,
        }
    }

    pub fn as_module_arc(&self) -> Option<Arc<Module>> {
        match self {
            Self::Module(m) => Some(m.clone()),
            _ => None,
        }
    }
}

impl From<Arc<Module>> for Symbol {
    fn from(module: Arc<Module>) -> Self {
        Symbol::Module(module)
    }
}

impl From<Arc<Link>> for Symbol {
    fn from(link: Arc<Link>) -> Self {
        Symbol::Link(link)
    }
}
