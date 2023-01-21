use des::{
    net::plugin2::{add_plugin, Plugin},
    prelude::*,
};

#[NdlModule("examples/plugin")]
struct A {}

impl Module for A {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::new().content(42).build(), "out")
    }
}

struct OutputLogger;
impl Plugin for OutputLogger {
    fn capture_outgoing(&mut self, msg: Message) -> Option<Message> {
        log::info!("sending: {}", msg.str());
        add_plugin(OutputLogger, 100);
        Some(msg)
    }
}

#[NdlModule("examples/plugin")]
struct B {}

impl Module for B {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin(OutputLogger, 10);
    }

    fn handle_message(&mut self, msg: Message) {
        send(msg, "out")
    }
}

#[NdlSubsystem("examples/plugin")]
struct Main {}

fn main() {
    Logger::new().set_logger();

    let app = Main {};
    let app = app.build_rt();
    let rt = Runtime::new(app);
    let _res = rt.run();
}
