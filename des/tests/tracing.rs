use des::{net::{Sim, AsyncFn}, runtime::Builder, tracing::format};
use tracing::{level_filters::LevelFilter, subscriber::with_default, Instrument, span, Level};

mod mock {
    use spin::Mutex;
    use std::{io, sync::Arc};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Debug, Clone)]
    pub struct MakeMockWriter {
        lines: Arc<Mutex<String>>,
    }

    #[derive(Debug, Clone)]
    pub struct MockWriter {
        lines: Arc<Mutex<String>>,
    }

    impl io::Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut lines = self.lines.lock();
            lines.push_str(&String::from_utf8_lossy(buf));
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl MakeMockWriter {
        pub fn new() -> Self {
            MakeMockWriter {
                lines: Arc::new(Mutex::new(String::new())),
            }
        }

        pub fn content(&self) -> String {
            self.lines.lock().clone()
        }
    }

    impl<'a> MakeWriter<'a> for MakeMockWriter {
        type Writer = MockWriter;
        fn make_writer(&'a self) -> Self::Writer {
            MockWriter {
                lines: self.lines.clone(),
            }
        }
    }
}

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
        }));

        let _ = Builder::seeded(123).build(sim).run();
        assert_eq!(
            writer.content(), 
            "[ 0ns ] INFO a my-span{key=123}:say_hello: tracing: hello\n[ 0ns ] TRACE a.b tracing: node(b) says(1) at(0s)\n"
        );
    });
}



