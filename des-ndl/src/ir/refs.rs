use super::*;

#[derive(Debug)]
pub struct GateRef<'a> {
    pub submod: Option<(usize, String, Cluster)>,
    pub def: &'a Gate,
    pub pos: Option<usize>,
}

#[derive(Debug)]
pub struct SubmoduleRef<'a> {
    pub def: &'a Submodule,
    pub pos: usize,
}
