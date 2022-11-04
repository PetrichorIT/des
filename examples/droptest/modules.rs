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
        let prev = MODULE_LEN.fetch_add(1, Ordering::SeqCst);
        println!("Alice simstared: MODULE_LEN := {}", prev + 1)
    }

    fn handle_message(&mut self, _: Message) {
        // let (msg, head) = msg.cast::<usize>();
        // println!("Received msg: {} - {:?}", msg, head);
    }
}

impl Drop for Alice {
    fn drop(&mut self) {
        println!("<DROP> Alice");
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
        let prev = MODULE_LEN.fetch_add(1, Ordering::SeqCst);
        println!("Bob simstared: MODULE_LEN := {}", prev + 1)
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, _) = msg.cast::<usize>();

        // println!("Received msg: {} - {:?}", msg, head);

        let msg = Message::new().kind(2).content(msg).build();
        send(msg, ("netOut", 0))
    }
}

impl Drop for Bob {
    fn drop(&mut self) {
        println!("<DROP> Bob");
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
        let prev = MODULE_LEN.fetch_add(1, Ordering::SeqCst);
        println!("Network simstared: MODULE_LEN := {}", prev + 1);
    }

    fn handle_message(&mut self, _: Message) {
        unimplemented!()
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        println!("<DROP> Network");
        MODULE_LEN.fetch_sub(1, Ordering::SeqCst);
    }
}
