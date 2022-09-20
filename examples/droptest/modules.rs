use std::sync::atomic::Ordering;

use crate::MODULE_LEN;
use des::prelude::*;

#[NdlModule("examples/droptest")]
pub struct Alice();

impl Module for Alice {
    fn new() -> Self {
        Self()
    }

    fn at_sim_start(&mut self, _: usize) {
        let msg = Message::new().kind(1).content(42usize).build();
        send(msg, ("netOut", 0));

        println!("SimStared");
        MODULE_LEN.fetch_add(1, Ordering::SeqCst);
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();
        println!("Received msg: {} - {:?}", msg, head);
    }
}

impl Drop for Alice {
    fn drop(&mut self) {
        MODULE_LEN.fetch_sub(1, Ordering::SeqCst);
    }
}

#[NdlModule("examples/droptest")]
pub struct Bob();

impl Module for Bob {
    fn new() -> Self {
        Self()
    }

    fn at_sim_start(&mut self, _stage: usize) {
        MODULE_LEN.fetch_add(1, Ordering::SeqCst);
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();

        println!("Received msg: {} - {:?}", msg, head);

        let msg = Message::new().kind(2).content(msg).build();
        send(msg, ("netOut", 0))
    }
}

impl Drop for Bob {
    fn drop(&mut self) {
        MODULE_LEN.fetch_sub(1, Ordering::SeqCst);
    }
}

#[NdlModule("examples/droptest")]
pub struct Network();

impl Module for Network {
    fn new() -> Self {
        Self()
    }

    fn at_sim_start(&mut self, _: usize) {
        MODULE_LEN.fetch_add(1, Ordering::SeqCst);
    }

    fn handle_message(&mut self, _: Message) {
        unimplemented!()
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        MODULE_LEN.fetch_sub(1, Ordering::SeqCst);
    }
}
