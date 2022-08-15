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

#[derive(MessageBody)]
struct ComposedType {
    a: u64,
    b: u32,
}

#[derive(MessageBody)]
enum ComposedEnum {
    A(ComposedType),
    B { str: String, b: u64 },
}

fn main() {
    let a = ComposedType { a: 0, b: 0 };
    assert_eq!(a.byte_len(), 12);

    let aa = ComposedEnum::A(a);
    assert_eq!(aa.byte_len(), 12);

    let b = ComposedEnum::B {
        str: "Hello World".to_string(),
        b: 0,
    };
    assert_eq!(b.byte_len(), 11 + 8)
}
