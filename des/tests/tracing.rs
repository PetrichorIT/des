use des::{Sim, handlers::AsyncHandler, tracing::format};
use tracing::{Instrument, Level, level_filters::LevelFilter, span, subscriber::with_default};

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

#[test]
#[serial_test::serial]
fn test_mock_output() {
    let writer = MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let sim = Sim::new(());
        let _ = sim.seeded(123).build().run();

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
    let writer = MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node(
            "a",
            AsyncHandler::new(|_| async {
                tracing::info!("node(a) says(1) at(0s)");
                tracing::error!("node(a) says(2) at(0s)");
            }),
        );
        sim.node(
            "a.b",
            AsyncHandler::new(|_| async {
                tracing::trace!("node(b) says(1) at(0s)");
            }),
        );

        let _ = sim.seeded(123).build().run();
        assert_eq!(
            writer.content(),
            "[ 0ns ] INFO a tracing: node(a) says(1) at(0s)\n[ 0ns ] ERROR a tracing: node(a) says(2) at(0s)\n[ 0ns ] TRACE a.b tracing: node(b) says(1) at(0s)\n"
        );
    });
}

#[test]
#[serial_test::serial]
fn time_regognition() {
    let writer = MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node(
            "a",
            AsyncHandler::new(|_| async {
                tracing::info!("node(a) says(1) at(0s)");
                des::time::sleep(std::time::Duration::from_secs(5)).await;
                tracing::error!("node(a) says(2) at(5s)");
            }),
        );
        sim.node(
            "a.b",
            AsyncHandler::new(|_| async {
                tracing::trace!("node(b) says(1) at(0s)");
            }),
        );

        let _ = sim.seeded(123).build().run();
        assert_eq!(
            writer.content(),
            "[ 0ns ] INFO a tracing: node(a) says(1) at(0s)\n[ 0ns ] TRACE a.b tracing: node(b) says(1) at(0s)\n[ 5s ] ERROR a tracing: node(a) says(2) at(5s)\n"
        );
    });
}

#[test]
#[serial_test::serial]
fn span_regognition() {
    let writer = MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node(
            "a",
            AsyncHandler::new(|_| {
                async {
                    tracing::info!("node(a) says(1) at(0s)");
                }
                .instrument(span!(Level::DEBUG, "my-span", key = 123))
            }),
        );
        sim.node(
            "a.b",
            AsyncHandler::new(|_| async {
                tracing::trace!("node(b) says(1) at(0s)");
            }),
        );

        let _ = sim.seeded(123).build().run();
        assert_eq!(
            writer.content(),
            "[ 0ns ] INFO a tracing: my-span{key=123}: node(a) says(1) at(0s)\n[ 0ns ] TRACE a.b tracing: node(b) says(1) at(0s)\n"
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

    let writer = MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node(
            "a",
            AsyncHandler::new(|_| {
                async {
                    say_hello().await;
                }
                .instrument(span!(Level::DEBUG, "my-span", key = 123))
            }),
        );
        sim.node(
            "a.b",
            AsyncHandler::new(|_| {
                async {
                    tracing::trace!("node(b) says(1) at(0s)");
                }
                .instrument(span!(Level::DEBUG, "other-span"))
            }),
        );

        let _ = sim.seeded(123).build().run();
        assert_eq!(
            writer.content(),
            "[ 0ns ] INFO a tracing: my-span{key=123}:say_hello: hello\n[ 0ns ] TRACE a.b tracing: other-span: node(b) says(1) at(0s)\n"
        );
    });
}

#[test]
#[serial_test::serial]
fn with_ansi() {
    let writer = MakeMockWriter::new();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(LevelFilter::TRACE)
        .event_format(format())
        .with_writer(writer.clone())
        .finish();

    with_default(subscriber, || {
        let mut sim = Sim::new(());
        sim.node(
            "a",
            AsyncHandler::new(|_| async { tracing::info!("Hello World!") }),
        );

        let _ = sim.seeded(123).build().run();
        assert_eq!(
            writer.content(),
            "\u{1b}[2m[ 0ns ] \u{1b}[0m\u{1b}[32ma \u{1b}[0m\u{1b}[2mtracing: \u{1b}[0mHello World!\n"
        );
    });
}
