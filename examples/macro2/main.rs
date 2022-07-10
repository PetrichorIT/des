use des::prelude::*;

#[NdlModule(workspace = "examples/macro2", ndl_ident = "Alice")]
struct AliceImpl {
    #[allow(dead_code)]
    some_state: usize,
}

// TODO: Find a solution for this
impl NameableModule for Alice {
    fn named(core: ModuleCore) -> Self {
        Self {
            some_state: 42,
            __core: core,
        }
    }
}

impl Module for Alice {
    fn handle_message(&mut self, _msg: Message) {}
}

#[NdlSubsystem(workspace = "examples/macro2")]
struct TestNet {
    #[allow(dead_code)]
    global_data: Vec<usize>,
}

#[NdlSubsystem("examples/macro2", "MainNet")]
struct MyNet;

fn main() {}
