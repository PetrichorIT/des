use des::prelude::*;

#[NdlModule("tests/macro2")]
struct Alice {
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

#[NdlSubsystem("tests/macro2")]
struct TestNet {
    #[allow(dead_code)]
    global_data: Vec<usize>,
}

fn main() {}
