use des::{net::{Sim, AsyncFn}, runtime::Builder, tracing::format};
use tracing::{level_filters::LevelFilter, subscriber::with_default, Instrument, span, Level};

#[path ="common/mock.rs"]
mod mock;

#[test]
#[serial_test::serial]
fn test_mock_output() {
    let writer = mock::MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let sim = Sim::new(());
        let _ = Builder::seeded(123).build(sim).run();

        tracing::info!(GENERAL = "Kenobi", "Hello there");
        assert_eq!(
            writer.content(),
            "[ 0ns ] INFO tracing: Hello there GENERAL=\"Kenobi\"\n"
        );
    })
}

#[test]
#[serial_test::serial]
fn scope_regognition() {
    let writer = mock::MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node("a", AsyncFn::new(|_| async {
            tracing::info!("node(a) says(1) at(0s)");
            tracing::error!("node(a) says(2) at(0s)");

        }));
        sim.node("a.b", AsyncFn::new(|_| async {
            tracing::trace!("node(b) says(1) at(0s)");
        }));

        let _ = Builder::seeded(123).build(sim).run();
        assert_eq!(
            writer.content(), 
            "[ 0ns ] INFO a tracing: node(a) says(1) at(0s)\n[ 0ns ] ERROR a tracing: node(a) says(2) at(0s)\n[ 0ns ] TRACE a.b tracing: node(b) says(1) at(0s)\n"
        );
    });
}

#[test]
#[serial_test::serial]
fn time_regognition() {
    let writer = mock::MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node("a", AsyncFn::new(|_| async {
            tracing::info!("node(a) says(1) at(0s)");
            des::time::sleep(std::time::Duration::from_secs(5)).await;
            tracing::error!("node(a) says(2) at(5s)");

        }));
        sim.node("a.b", AsyncFn::new(|_| async {
            tracing::trace!("node(b) says(1) at(0s)");
        }));

        let _ = Builder::seeded(123).build(sim).run();
        assert_eq!(
            writer.content(), 
            "[ 0ns ] INFO a tracing: node(a) says(1) at(0s)\n[ 0ns ] TRACE a.b tracing: node(b) says(1) at(0s)\n[ 5s ] ERROR a tracing: node(a) says(2) at(5s)\n"
        );
    });
}

#[test]
#[serial_test::serial]
fn span_regognition() {
    let writer = mock::MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node("a", AsyncFn::new(|_| async {
            tracing::info!("node(a) says(1) at(0s)");
        }.instrument(span!(Level::DEBUG, "my-span", key=123))));
        sim.node("a.b", AsyncFn::new(|_| async {
            tracing::trace!("node(b) says(1) at(0s)");
        }));

        let _ = Builder::seeded(123).build(sim).run();
        assert_eq!(
            writer.content(), 
            "[ 0ns ] INFO a my-span{key=123}: tracing: node(a) says(1) at(0s)\n[ 0ns ] TRACE a.b tracing: node(b) says(1) at(0s)\n"
        );
    });
}

#[test]
#[serial_test::serial]
fn multi_span_regognition() {
    #[tracing::instrument]
    async fn say_hello() {
        tracing::info!("hello")
    }
    
    let writer = mock::MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node("a", AsyncFn::new(|_| async {
            say_hello().await;
        }.instrument(span!(Level::DEBUG, "my-span", key=123))));
        sim.node("a.b", AsyncFn::new(|_| async {
            tracing::trace!("node(b) says(1) at(0s)");
        }.instrument(span!(Level::DEBUG, "other-span"))));

        let _ = Builder::seeded(123).build(sim).run();
        assert_eq!(
            writer.content(), 
            "[ 0ns ] INFO a my-span{key=123}:say_hello: tracing: hello\n[ 0ns ] TRACE a.b other-span: tracing: node(b) says(1) at(0s)\n"
        );
    });
}

#[test]
#[serial_test::serial]
fn with_ansi() {
    #[tracing::instrument]
    async fn say_hello() {
        tracing::info!("hello")
    }
    
    let writer = mock::MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node("a", AsyncFn::new(|_| async {
            tracing::info!("Hello World!")
        }));
       

        let _ = Builder::seeded(123).build(sim).run();
        assert_eq!(
            writer.content(), 
            "\u{1b}[2m[ 0ns ] \u{1b}[0m\u{1b}[32ma \u{1b}[0m\u{1b}[2mtracing: \u{1b}[0mHello World!\n"
        );
    });
}



