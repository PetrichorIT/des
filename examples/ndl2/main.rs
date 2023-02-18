use des::{
    ndl::{NdlApplication, Registry},
    prelude::*,
};

struct Sub;
impl Module for Sub {
    fn new() -> Self {
        log::debug!("# create sub");
        Self
    }

    fn at_sim_start(&mut self, _stage: usize) {
        if module_name() == "a" {
            send(Message::new().build(), "out");
        }
    }

    fn handle_message(&mut self, msg: Message) {
        send(msg, "out");
        log::info!("EY");
    }
}

struct Main;
impl Module for Main {
    fn new() -> Self {
        log::debug!("# create main");
        Self
    }
}

fn main() {
    Logger::new()
        .interal_max_log_level(log::LevelFilter::Debug)
        .set_logger();

    let mut registry = Registry::new();
    registry.add("Sub", Box::new(|| Box::new(Sub::new())));
    registry.add("Main", Box::new(|| Box::new(Main::new())));

    let ndl = match NdlApplication::new("examples/ndl2/main.ndl", registry) {
        Ok(v) => v,
        Err(e) => {
            println!("{e}");
            panic!("exiting due to previouis error")
        }
    };
    let app = NetworkRuntime::new(ndl);
    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123).max_itr(10));
    let _ = rt.run();
}
