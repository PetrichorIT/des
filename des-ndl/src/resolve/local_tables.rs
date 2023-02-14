use super::IterIter;
use crate::{
    ast::{LocalModuleGateReference, ModuleGateReference, NonlocalModuleGateReference},
    error::*,
    ir::*,
};
use std::iter;

pub struct LocalGatesTable<'a> {
    gates: &'a [Gate],
}

impl<'a> LocalGatesTable<'a> {
    pub fn new(gates: &'a [Gate]) -> Self {
        Self { gates }
    }

    pub fn resolve(
        &self,
        gatec_def: &'a LocalModuleGateReference,
    ) -> Result<Box<dyn Iterator<Item = GateRef<'a>> + 'a>> {
        let Some(def) = self
            .gates
            .iter()
            .find(|g| g.ident.raw == gatec_def.gate.raw) else {
                return Err(Error::new(
                    ErrorKind::SymbolNotFound,
                    format!("did not find gate symbol '{}', not in scope", gatec_def.gate.raw)
                ))
            };
        match (&gatec_def.gate_cluster, def.cluster) {
            (None, Cluster::Standalone) => Ok(Box::new(iter::once(GateRef { def, pos: None }))),
            (None, Cluster::Clusted(cl)) => Ok(Box::new((0..cl).map(|pos| GateRef {
                def,
                pos: Some(pos),
            }))),
            (Some(c), Cluster::Clusted(cl)) if (c.lit.as_integer() as usize) < cl => {
                Ok(Box::new(iter::once(GateRef {
                    def,
                    pos: Some(c.lit.as_integer() as usize),
                })))
            }
            (Some(c), Cluster::Clusted(cl)) => Err(Error::new(
                ErrorKind::InvalidConClusterIndex,
                format!(
                    "cannot index into gate cluster of size {} with index {}",
                    cl,
                    c.lit.as_integer()
                ),
            )),
            (Some(_), Cluster::Standalone) => Err(Error::new(
                ErrorKind::InvalidConClusterIndex,
                format!(
                    "cannot index into gate '{}' since it is not a cluster",
                    def.ident.raw
                ),
            )),
        }
    }
}

pub struct SharedGatesTable<'a> {
    local: &'a LocalGatesTable<'a>,
    submodules: &'a LocalSubmoduleTable<'a>,
}

impl<'a> SharedGatesTable<'a> {
    pub fn new(local: &'a LocalGatesTable<'a>, submodules: &'a LocalSubmoduleTable<'a>) -> Self {
        Self { local, submodules }
    }

    pub fn resolve(
        &self,
        def: &'a ModuleGateReference,
    ) -> Result<Box<dyn Iterator<Item = GateRef<'a>> + 'a>> {
        match def {
            ModuleGateReference::Local(gatec_def) => self.local.resolve(gatec_def),
            ModuleGateReference::Nonlocal(submodgate_def) => {
                let submodules = self.submodules.resolve(submodgate_def)?;
                let mut gates_iters = Vec::new();
                for submodule in submodules {
                    let Some(submodule) = submodule.def.typ.as_module() else {
                        // type of submodule is not specified, thus ignore this error as transitent.
                        unimplemented!()
                    };
                    let tbl = LocalGatesTable::new(&submodule.gates);
                    let gates = tbl.resolve(&submodgate_def.gate)?;
                    gates_iters.push(gates);
                }

                Ok(Box::new(IterIter::new(gates_iters.into_iter())))
            }
        }
    }
}

pub struct LocalSubmoduleTable<'a> {
    modules: &'a [Submodule],
}

impl<'a> LocalSubmoduleTable<'a> {
    pub fn new(modules: &'a [Submodule]) -> Self {
        Self { modules }
    }

    pub fn resolve(
        &self,
        submodgate_def: &'a NonlocalModuleGateReference,
    ) -> Result<Box<dyn Iterator<Item = SubmoduleRef<'a>> + 'a>> {
        let Some(def) = self
            .modules
            .iter()
            .find(|def| def.ident.raw == submodgate_def.submodule.raw) else {
                return Err(Error::new(
                    ErrorKind::SymbolNotFound,
                    format!("did not find submodule symbol '{}', not in scope", submodgate_def.submodule.raw)
                ))
        };

        match (&submodgate_def.submodule_cluster, def.cluster) {
            (None, Cluster::Standalone) => Ok(Box::new(iter::once(SubmoduleRef { def, pos: 0 }))),
            (None, Cluster::Clusted(cl)) => {
                Ok(Box::new((0..cl).map(|pos| SubmoduleRef { def, pos })))
            }
            (Some(c), Cluster::Clusted(cl)) if (c.lit.as_integer() as usize) < cl => {
                Ok(Box::new(iter::once(SubmoduleRef {
                    def,
                    pos: c.lit.as_integer() as usize,
                })))
            }
            (Some(c), Cluster::Clusted(cl)) => Err(Error::new(
                ErrorKind::InvalidConClusterIndex,
                format!(
                    "cannot index into submdoule cluster of size {} with index {}",
                    cl,
                    c.lit.as_integer()
                ),
            )),
            (Some(_), Cluster::Standalone) => Err(Error::new(
                ErrorKind::InvalidConClusterIndex,
                format!(
                    "cannot index into submodule '{}' since it is not a cluster",
                    def.ident.raw
                ),
            )),
        }
    }
}
