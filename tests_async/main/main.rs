use std::sync::Arc;

use des::prelude::*;
use des::sync::mpsc::*;
use tokio::task::JoinHandle;

#[derive(Debug, Module)]
#[ndl_workspace = "main"]
pub struct M {
    core: ModuleCore,
    rt: tokio::runtime::Runtime,
    handles: Vec<JoinHandle<()>>,
    sender: Option<des::sync::mpsc::Sender<u32>>,
    globals: Arc<Globals>,
}

impl NameableModule for M {
    fn named(core: ModuleCore) -> Self {
        Self {
            core,
            rt: tokio::runtime::Runtime::new().unwrap(),
            handles: Vec::new(),
            sender: None,
            globals: Globals::new(),
        }
    }
}

impl Module for M {
    fn at_sim_start(&mut self, _stage: usize) {
        let out = self.gate("out", 0).unwrap();
        let handle = self.async_buffers();
        let (tx, mut rx) = channel_watched(32, self.globals.clone());

        let handle = self.rt.spawn(async move {
            while let Some(msg) = rx.recv().await {
                handle
                    .send(Message::new().content(msg).build(), out.clone())
                    .await;
            }
        });

        self.sender = Some(tx);
        self.handles.push(handle);

        self.schedule_in(Message::new().content(1).build(), Duration::from_secs(1))
    }

    fn handle_message(&mut self, _msg: Message) {
        let sender = self.sender.as_ref().unwrap().clone();
        self.rt.spawn(async move {
            println!("notify");
            sender.send(42).await.unwrap();
        });

        self.rt.block_on(self.globals.notifed());
        println!("Got Notified")
    }
}

#[derive(Debug, Network)]
#[ndl_workspace = "main"]
pub struct N {}

fn main() {
    let n = N {};
    let _res = n.run_with_options(RuntimeOptions::seeded(1).max_time(10.0.into()));
}
