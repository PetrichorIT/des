use crate::{
    ir::*, Annotation, ClusterDefinition, Ident, LocalModuleGateReference, ModuleGateReference,
};

pub struct LocalGatesTable<'a> {
    gates: &'a [Gate],
}

impl<'a> LocalGatesTable<'a> {
    pub fn new(gates: &'a [Gate]) -> Self {
        Self { gates }
    }

    pub fn exists(&self, symbol: RawSymbol, cluster: Cluster) -> bool {
        self.gates
            .iter()
            .any(|g| g.ident == symbol && g.cluster.contains(&cluster))
    }

    pub fn get(&self, local: &LocalModuleGateReference) -> Option<&Gate> {
        self.gates.iter().find(|g| g.ident.raw == local.gate.raw)
    }
}

pub struct SharedGatesTable<'a> {
    local: &'a LocalGatesTable<'a>,
    submodules: &'a LocalSubmoduleTable<'a>,
}

impl<'a> SharedGatesTable<'a> {
    pub fn get(&self, mgref: &ModuleGateReference) -> Option<&'a Gate> {
        match mgref {
            ModuleGateReference::Local(mgref) => self.local.get(mgref),
            ModuleGateReference::Nonlocal(mgref) => {
                let submodule = self
                    .submodules
                    .get(&mgref.submodule, &mgref.submodule_cluster)?;

                let module = submodule.typ.as_module()?;
                module.gates.iter().find(|g| g.ident.raw == mgref.gate.raw)
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

    pub fn get(&self, ident: &Ident, cluster: &Option<ClusterDefinition>) -> Option<&Submodule> {
        let cluster = cluster
            .as_ref()
            .map(Cluster::from)
            .unwrap_or(Cluster::Standalone);

        self.modules
            .iter()
            .find(|m| m.ident.could_be_submodule(ident, &cluster))
    }
}
