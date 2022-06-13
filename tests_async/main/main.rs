use std::sync::Arc;

use des::prelude::*;
use des::sync::mpsc::*;

#[derive(Debug, Module)]
#[ndl_workspace = "main"]
pub struct M {
    core: ModuleCore,
    rt: tokio::runtime::Runtime,
    globals: Arc<Globals>,

    to_async_sender: Sender<u32>,
    to_async_receiver: Option<Receiver<u32>>,

    from_async_sender: UnboundedSender<u32>,
    from_async_receiver: UnboundedReceiver<u32>,
}

impl NameableModule for M {
    fn named(core: ModuleCore) -> Self {
        let globals = Globals::new();
        let (tx, rx) = des::sync::mpsc::channel_watched(32, globals.clone());
        let (ty, ry) = des::sync::mpsc::unbounded_channel_unwatched();

        Self {
            core,
            rt: tokio::runtime::Runtime::new().unwrap(),
            globals,

            to_async_sender: tx,
            to_async_receiver: Some(rx),

            from_async_sender: ty,
            from_async_receiver: ry,
        }
    }
}

impl Module for M {
    fn at_sim_start(&mut self, _stage: usize) {
        let mut rx = self.to_async_receiver.take().unwrap();
        let tx = self.from_async_sender.clone();
        self.rt.spawn(async move {
            let tx = &tx;
            loop {
                rx.scoped_recv(|v| async move {
                    let v = v.unwrap();
                    println!("got message {} ... sending {} and {}", v, v + 1, v + 2);
                    tx.send(v + 1).unwrap();
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

        while let Ok(v) = self.from_async_receiver.try_recv() {
            self.send(Message::new().content(v).build(), "out")
        }
    }
}

#[derive(Debug, Network)]
#[ndl_workspace = "main"]
pub struct N {}

fn main() {
    let n = N {};
    let _res = n.run_with_options(RuntimeOptions::seeded(1).max_time(10.0.into()));
}
