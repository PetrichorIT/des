use des::{prelude::*, registry};

#[derive(Default)]
struct Sub;
impl Module for Sub {
    fn at_sim_start(&mut self, _stage: usize) {
        if current().name() == "a" {
            send(Message::new().build(), "out");
        }
    }

    fn handle_message(&mut self, msg: Message) {
        send(msg, "out");
        tracing::info!("EY");
    }
}

#[derive(Default)]
struct Main;
impl Module for Main {
    fn at_sim_end(&mut self) {
        tracing::info!(target: "custom", "at sim end")
    }
}

fn main() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Debug)
    //     .set_logger();

    let app = match Sim::ndl2("examples/ndl2/main.yml", registry![Main, Sub]) {
        Ok(v) => v,
        Err(e) => {
            println!("{e}");
            panic!("exiting due to previouis error")
        }
    };
    let rt = Builder::seeded(123).max_itr(10).build(app);
    let _ = rt.run();
}
