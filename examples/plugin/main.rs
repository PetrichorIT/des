use des::{
    net::{
        module::{set_setup_fn, ModuleContext},
        plugin::{add_plugin, Plugin, PluginHandle, PluginStatus},
    },
    prelude::*,
    registry,
};

struct A {}

impl Module for A {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::new().content(42).build(), "out");
        send(Message::new().content(69).build(), "out");
    }

    fn handle_message(&mut self, msg: Message) {
        log::info!("recv: {} {}", msg.str(), msg.content::<i32>())
    }
}

struct Dummy;
impl Plugin for Dummy {}

struct OutputLogger {
    handle: Option<PluginHandle>,
}
impl Plugin for OutputLogger {
    fn capture_outgoing(&mut self, msg: Message) -> Option<Message> {
        log::info!("sending: {}", msg.str());
        match self.handle.take() {
            Some(h) => {
                assert_eq!(h.status(), PluginStatus::Active);
                h.remove();
            }
            None => {
                self.handle = Some(add_plugin(Dummy, 1000));
                assert_eq!(
                    self.handle.as_ref().unwrap().status(),
                    PluginStatus::StartingUp
                );
            }
        }
        send(msg.dup::<i32>(), "out");
        Some(msg)
    }
}

struct B {}

impl Module for B {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin(Dummy, 1);
        add_plugin(OutputLogger { handle: None }, 10);
        add_plugin(Dummy, 100);
        add_plugin(Dummy, 2);
    }

    fn handle_message(&mut self, msg: Message) {
        send(msg, "out")
    }
}

struct Main;
impl Module for Main {
    fn new() -> Self {
        Self
    }
}

fn empty(_: &ModuleContext) {}

fn main() {
    Logger::new().set_logger();

    set_setup_fn(empty);

    let app = NdlApplication::new("examples/plugin/main.ndl", registry![A, B, Main]).unwrap();
    let app = NetworkApplication::new(app);
    let rt = Runtime::new(app);
    let _res = rt.run();
}
