use std::{thread, time::Duration};

use des::{
    net::{Failure, Sim, handlers::AsyncHandler},
    runtime::Builder,
    time::SimTime,
};
use serial_test::serial;
use tokio::time::Instant;

#[serial]
#[test]
fn instant_to_simtime() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::once(|_| async move {
            assert_eq!(SimTime::now(), 0.0);
            assert_eq!(SimTime::from_instant(Instant::now()), 0.0);
            assert_eq!(
                Instant::now(),
                SimTime::from_instant(Instant::now()).to_instant()
            );

            des::time::sleep(Duration::from_secs(2)).await;

            assert_eq!(SimTime::now(), 2.0);
            assert_eq!(SimTime::from_instant(Instant::now()), 2.0);
            assert_eq!(
                Instant::now(),
                SimTime::from_instant(Instant::now()).to_instant()
            );

            tokio::time::sleep(Duration::from_secs(2)).await;

            assert_eq!(SimTime::now(), 4.0);
            assert_eq!(SimTime::from_instant(Instant::now()), 4.0);
            assert_eq!(
                Instant::now(),
                SimTime::from_instant(Instant::now()).to_instant()
            );
        }),
    );

    Builder::seeded(123).build(sim.freeze()).run().as_result()?;
    Ok(())
}

#[serial]
#[test]
fn different_runner_different_start_time() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    let (tx, rx) = std::sync::mpsc::channel();
    let tx2 = tx.clone();

    sim.node(
        "alice",
        AsyncHandler::once(|_| async move {
            tx.send((Instant::now(), SimTime::from_instant(Instant::now())))
                .unwrap();
        }),
    );

    thread::sleep(Duration::from_millis(100));

    sim.node(
        "bob",
        AsyncHandler::once(|_| async move {
            tx2.send((Instant::now(), SimTime::from_instant(Instant::now())))
                .unwrap();
        }),
    );

    Builder::seeded(123).build(sim.freeze()).run().as_result()?;

    let (t0, s0) = rx.try_recv().unwrap();
    let (t1, s1) = rx.try_recv().unwrap();

    assert_ne!(t0, t1);
    assert_eq!(s0, s1);

    Ok(())
}

#[serial]
#[test]
fn instant_clock_paused() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::once(|_| async move {
            let t0 = Instant::now();
            thread::sleep(Duration::from_millis(100));
            let t1 = Instant::now();
            assert_eq!(t0, t1);
        }),
    );

    Builder::seeded(123).build(sim.freeze()).run().as_result()?;

    Ok(())
}
