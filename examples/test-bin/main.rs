use des::net::AsyncFn;
use des::prelude::*;
use des::time;

fn main() {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncFn::new(|_| async move {
            tokio::select! {
                _ = time::sleep(Duration::from_secs(10)) => unreachable!(),
                _ = time::sleep(Duration::from_secs(5)) => println!("resolved"),
            }
        })
        .require_join(),
    );

    let _ = Builder::seeded(123).build(sim).run();
    todo!()
}
