use std::fmt::Display;

use crate::AssetDescriptor;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OIdent {
    typ: OType,
    asset: AssetDescriptor,
    raw: String,
}

impl OIdent {
    pub fn asset(&self) -> &AssetDescriptor {
        &self.asset
    }

    pub fn typ(&self) -> OType {
        self.typ
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn new(typ: OType, asset: AssetDescriptor, raw: String) -> OIdent {
        Self { typ, asset, raw }
    }

    pub fn cast_type(&self, typ: OType) -> OIdent {
        let mut clone = self.clone();
        clone.typ = typ;
        clone
    }

    pub fn module(ident: String, asset: AssetDescriptor) -> OIdent {
        Self {
            typ: OType::Module,
            asset,
            raw: ident,
        }
    }

    pub fn subsystem(ident: String, asset: AssetDescriptor) -> OIdent {
        Self {
            typ: OType::Subsystem,
            asset,
            raw: ident,
        }
    }
}

impl Display for OIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}@{}>", self.raw, self.typ, self.asset.alias)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OType {
    Link,
    Prototype,
    Module,
    Alias,
    Subsystem,
}

impl Display for OType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Link => write!(f, "link"),
            Self::Prototype => write!(f, "prototype"),
            Self::Module => write!(f, "module"),
            Self::Alias => write!(f, "alias"),
            Self::Subsystem => write!(f, "subsystem"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateAnnotation {
    Unknown,
    Input,
    Output,
}

impl Display for GateAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, ""),
            Self::Input => write!(f, "@input"),
            Self::Output => write!(f, "@output"),
        }
    }
}
