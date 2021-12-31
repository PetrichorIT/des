use des_core::*;
use des_macros::Module;

#[derive(Module)]
#[ndl_workspace = "src"]
struct SimpleModule {
    core: ModuleCore,
}

fn main() {
    println!("Hello, world!");
}