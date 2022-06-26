use std::sync::Arc;

use des::prelude::*;
use des::sync::mpsc::*;

#[NdlModule("main")]
pub struct M {
    core: ModuleCore,
    rt: tokio::runtime::Runtime,
    globals: Arc<Globals>,

    to_async_sender: Sender<u32>,
    to_async_receiver: Option<Receiver<u32>>,
}

impl NameableModule for M {
    fn named(core: ModuleCore) -> Self {
        let globals = Globals::new();
        let (tx, rx) = des::sync::mpsc::channel_watched(32, globals.clone());

        Self {
            core,
            rt: tokio::runtime::Runtime::new().unwrap(),
            globals,

            to_async_sender: tx,
            to_async_receiver: Some(rx),
        }
    }
}

impl Module for M {
    fn at_sim_start(&mut self, _stage: usize) {
        let mut rx = self.to_async_receiver.take().unwrap();
        // let tx = self.from_async_sender.clone();

        let handle = self.async_handle();

        self.rt.spawn(async move {
            let handle = &handle;
            loop {
                rx.scoped_recv(|v| async move {
                    let v = v.unwrap();
                    println!("got message {} ... sending {} and {}", v, v + 1, v + 2);
                    handle.send(Message::new().content(v + 1).build(), "out");
                    // tx.send(v + 2).await.unwrap(); do not send else busy channel
                })
                .await;

                println!("Done with scoped");
            }
        });

        self.schedule_in(Message::new().content(1u32).build(), Duration::from_secs(1))
    }

    fn handle_message(&mut self, msg: Message) {
        let (content, _meta) = msg.cast::<u32>();
        self.to_async_sender.blocking_send(content).unwrap();

        self.rt.block_on(self.globals.notifed());
        println!("Got Notified");
    }
}

#[NdlSubsystem("main")]
#[derive(Debug, Default)]
pub struct N {}

fn main() {
    let n = N::default();
    let _res = n.run_with_options(RuntimeOptions::seeded(1).max_time(10.0.into()));
}
