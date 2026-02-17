use des::{net::Error, prelude::*};
use des_ndl::{SimExt, registry};

#[derive(Default)]
struct Sub;
impl Module for Sub {
    fn at_sim_start(&mut self, _stage: usize) {
        if current().name() == "a" {
            let _ = send(Message::default(), "out");
        }
    }

    fn handle_message(&mut self, msg: Message) {
        let _ = send(msg, "out");
        tracing::info!("EY");
    }
}

#[derive(Default)]
struct Main;
impl Module for Main {
    fn at_sim_end(&mut self) -> Result<(), Error> {
        tracing::info!(target: "custom", "at sim end");
        Ok(())
    }
}

fn main() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Debug)
    //     .set_logger();

    let app = match Sim::ndl("examples/ndl2/main.yml", registry![Main, Sub]) {
        Ok(v) => v,
        Err(e) => {
            println!("{e}");
            panic!("exiting due to previouis error")
        }
    };
    let rt = Builder::seeded(123).max_itr(10).build(app.freeze());
    let _ = rt.run();
}
