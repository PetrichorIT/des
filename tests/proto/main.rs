use des::prelude::*;
use des_derive::{Module, Network};

#[derive(Debug, Module)]
#[ndl_workspace = "tests/proto"]
struct AppA {
    core: ModuleCore,
}

impl Module for AppA {
    fn handle_message(&mut self, _msg: Message) {}
}

#[derive(Debug, Module)]
#[ndl_workspace = "tests/proto"]
struct AppB {
    core: ModuleCore,
}

impl Module for AppB {
    fn handle_message(&mut self, _msg: Message) {}
}

#[derive(Debug, Module)]
#[ndl_workspace = "tests/proto"]
struct Runner {
    core: ModuleCore,
}

impl Module for Runner {
    fn handle_message(&mut self, _msg: Message) {}
}

#[derive(Debug, Module)]
#[ndl_workspace = "tests/proto"]
struct MultiRunner {
    core: ModuleCore,
}

impl Module for MultiRunner {
    fn handle_message(&mut self, _msg: Message) {}
}

#[derive(Debug, Network)]
#[ndl_workspace = "tests/proto"]
struct Main();
fn main() {
    let app: NetworkRuntime<Main> = Main().build_rt();

    println!("{:?}", app.globals().parameters);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));
    let (app, _time, _event_count) = rt.run().unwrap();

    let _ = app
        .globals()
        .topology
        .write_to_svg("tests/proto/graph")
        .unwrap();
}
