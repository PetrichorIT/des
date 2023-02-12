use std::{ops::Deref, sync::Arc};

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct RawSymbol {
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    Module(MSymbol),
    Link(LSymbol),
    Unresolved,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MSymbol {
    module: Arc<Module>,
}

impl Deref for MSymbol {
    type Target = Module;
    fn deref(&self) -> &Self::Target {
        &self.module
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LSymbol {
    link: Arc<Link>,
}

impl Deref for LSymbol {
    type Target = Link;
    fn deref(&self) -> &Self::Target {
        &self.link
    }
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
}

impl From<Arc<Module>> for Symbol {
    fn from(module: Arc<Module>) -> Self {
        Symbol::Module(MSymbol { module })
    }
}

impl From<Arc<Link>> for Symbol {
    fn from(link: Arc<Link>) -> Self {
        Symbol::Link(LSymbol { link })
    }
}
