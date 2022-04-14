use des::prelude::*;
use des_derive::{Module, Network};

#[derive(Debug, Module)]
#[ndl_workspace = "tests/proto"]
struct AppA {
    core: ModuleCore,
}

impl Module for AppA {
    fn handle_message(&mut self, _msg: Message) {
        // println!("A: [{}] {:?}", SimTime::now(), _msg);
        assert_eq!(SimTime::now(), 2.0);
    }
}

#[derive(Debug, Module)]
#[ndl_workspace = "tests/proto"]
struct AppB {
    core: ModuleCore,
}

impl Module for AppB {
    fn handle_message(&mut self, _msg: Message) {
        // println!("B: [{}] {:?}", SimTime::now(), _msg);
        assert_eq!(SimTime::now(), 1.0);
    }
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
    fn at_sim_start(&mut self, _stage: usize) {
        self.schedule_at(Message::new().kind(42).build(), 1.0.into());
    }

    fn handle_message(&mut self, msg: Message) {
        // println!("M: [{}] {:?}", SimTime::now(), msg);
        if msg.meta().kind == 42 {
            self.send(msg.clone(), ("toAppl", 1));
            self.processing_time(1.0.into());
            self.send(msg, ("toAppl", 2));
            self.schedule_in(Message::new().kind(69).build(), 1.0.into());
        } else {
            assert_eq!(SimTime::now(), 2.0);
        }
    }
}

#[derive(Debug, Network)]
#[ndl_workspace = "tests/proto"]
struct Main();
fn main() {
    let app: NetworkRuntime<Main> = Main().build_rt();

    // println!("{:?}", app.globals().parameters);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));
    let (app, _time, _event_count) = rt.run().unwrap();

    let _ = app
        .globals()
        .topology
        .write_to_svg("tests/proto/graph")
        .unwrap();
}
