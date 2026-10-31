use des::{Error, prelude::*};
use des_ndl::{Ndl, registry};

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
    let mut app = Sim::new(());
    app.node(
        "",
        Ndl::from_str(&mut registry![Main, Sub], include_str!("main.yml")).unwrap(),
    )
    .unwrap();
    let rt = app.seeded(123).max_itr(10).build();
    let _ = rt.run();
}
