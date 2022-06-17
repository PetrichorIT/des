use super::{
    ctx::{GlobalTyComposeContext, TyComposeContext},
    prepare::{PrepareModule, PrepareSubsystem, PreparedUnit},
    specs::{ConSpecNodeIdent, ModuleSpec, SubsystemSpec},
};
use crate::{Error, NdlResolver};
use std::collections::HashMap;

pub type ComposedModule = ModuleSpec<ConSpecNodeIdent>;
pub type ComposedSubsystem = SubsystemSpec<ConSpecNodeIdent>;

type GlobalTySpecContext = ();

struct Ctx<'r> {
    // Reference context for compose stage.
    tyctx: TyComposeContext,
    gtyctx: GlobalTyComposeContext,

    // Output
    outctx: GlobalTySpecContext,

    resolver: &'r mut NdlResolver,

    error: Vec<Error>,
}

impl<'r> Ctx<'r> {
    pub fn process(&mut self) {}
}

enum Poll {
    Done,
    Pending(String),
}

enum GraphElement<'a> {
    Module(&'a PrepareModule),
    ModuleProcessed(ComposedModule),

    Subsystem(&'a PrepareSubsystem),
    SubsystemProcessed(ComposedSubsystem),
}

impl<'a> GraphElement<'a> {
    pub fn is_done(&self) -> bool {
        matches!(self, Self::ModuleProcessed(_) | Self::SubsystemProcessed(_))
    }
    pub fn process(&mut self) -> Poll {
        if self.is_done() {
            return Poll::Done;
        }

        match self {
            Self::Module(ptr) => *self = Self::ModuleProcessed(Self::process_module(ptr)),
            Self::Subsystem(ptr) => match Self::ensure_depenencies(ptr) {
                Poll::Pending(s) => return Poll::Pending(s),
                Done => *self = Self::SubsystemProcessed(Self::process_subsystem(ptr)),
            },
            _ => unreachable!(),
        };

        Poll::Done
    }

    fn process_module(this: &'a PrepareModule) -> ComposedModule {
        todo!()
    }

    fn ensure_depenencies(this: &'a PrepareSubsystem) -> Poll {
        todo!()
    }

    fn process_subsystem(this: &'a PrepareSubsystem) -> ComposedSubsystem {
        todo!()
    }
}

pub fn compose(
    units: &mut HashMap<String, PreparedUnit>,
    resolver: &mut NdlResolver,
    cycles: bool,
) {
}
