use std::io::Write;

use des::{prelude::*, registry};

#[derive(Debug, Default)]
struct A;
impl Module for A {}

#[derive(Debug, Default)]
struct Alice;
#[derive(Debug, Default)]
struct Bob;

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let content = msg.body.content::<String>();
        let limit = current().prop::<usize>("limit").unwrap().or_default().get();

        if content.len() <= limit {
            tracing::info!("returning event with {}", content.len());
            let _ = send(msg, "up");
        }
    }
}

impl Module for Bob {
    fn at_sim_start(&mut self, _stage: usize) {
        let _ = send(Message::default().with_content(String::new()), "down");
    }

    fn handle_message(&mut self, mut msg: Message) {
        let char = current().prop::<String>("char").unwrap().get().unwrap();
        msg.body.content_mut::<String>().push_str(&char);
        tracing::info!(
            "dispatching event with {}",
            msg.body.content::<String>().len()
        );
        let _ = send(msg, "down");
    }
}

const CFG: &str = r#"
bob[0].char: '#'
bob[1].char: '*'

bob[0].child.limit: 10
bob[1].child.limit: 2
"#;

fn main() -> std::io::Result<()> {
    let mut app = Sim::ndl("examples/utils/main.yml", registry![A, Alice, Bob])
        .map_err(|e| println!("{e}"))
        .unwrap();
    app.include_cfg(CFG);

    let rt = Builder::seeded(0x123).quiet().build(app.freeze());
    let (app, time, p) = rt.run().unwrap();

    let topo = app.globals().topology();

    assert_eq!(topo.nodes().len(), 5);
    assert_eq!(topo.edges().count(), 4);

    std::fs::File::create("examples/utils/graph.svg")?.write_all(topo.as_svg()?.as_bytes())?;

    // Chain 0: iterations [0, 1, 2, ..., 10] a [ChannelUnbusy, ExitingConn, HandleMessage] + 3
    // Chain 1: iterations [0, 1, 2] a 2 events + one 3th event
    assert_eq!(p.event_count, ((6 * 11 + 3) + (6 * 3 + 3)));

    // Chain 0 longest:
    // - start at 1
    // - 11 round trips
    assert_eq!(time.as_secs(), 2);

    Ok(())
}
