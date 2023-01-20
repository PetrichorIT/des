use std::sync::atomic::Ordering::SeqCst;
use std::sync::{atomic::AtomicUsize, Arc, Barrier};
use std::thread;

use des::prelude::*;

fn main() {
    let n = 5;
    let mut handles = Vec::with_capacity(n);

    let barrier = Arc::new(Barrier::new(n));
    let active = Arc::new(AtomicUsize::new(0));
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..n {
        let barrier_c = barrier.clone();
        let active_c = active.clone();
        let counter_c = counter.clone();

        handles.push(thread::spawn(move || {
            create_runtime_and_wait(barrier_c, active_c, counter_c)
        }))
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(active.load(SeqCst), 0);
    assert_eq!(counter.load(SeqCst), n)
}

fn create_runtime_and_wait(
    barrier: Arc<Barrier>,
    active: Arc<AtomicUsize>,
    counter: Arc<AtomicUsize>,
) {
    barrier.wait();

    // Create runtime
    let rt = Runtime::new(NetworkRuntime::new(()));
    let prev = active.fetch_add(1, SeqCst);
    assert_eq!(prev, 0);
    counter.fetch_add(1, SeqCst);

    // Do work
    thread::sleep(Duration::from_millis(500));

    // Deregister
    let prev = active.fetch_sub(1, SeqCst);
    assert_eq!(prev, 1);

    drop(rt);
}
