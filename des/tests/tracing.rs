use std::{sync::Arc, time::Duration};

use des::{
    net::AsyncBuilder,
    runtime::Builder,
    time::sleep,
    tracing::{
        NoColorFormatter, ScopeConfiguration, ScopeConfigurationPolicy, Subscriber, TracingOutput,
    },
};
use spin::Mutex;
use termcolor::{BufferWriter, ColorChoice};
use tracing::{Instrument, Level};

struct DebugOutput {
    records: Arc<Mutex<Vec<String>>>,
}
impl TracingOutput for DebugOutput {
    fn write(
        &mut self,
        fmt: &mut dyn des::tracing::TracingFormatter,
        record: des::tracing::TracingRecord<'_>,
    ) -> std::io::Result<()> {
        let wrt = BufferWriter::stdout(ColorChoice::Never);
        let mut buf = wrt.buffer();
        fmt.fmt(&mut buf, record)?;
        self.records
            .lock()
            .push(String::from_utf8_lossy(buf.as_slice()).to_string());
        Ok(())
    }
}

struct DebugPolicy {
    output: Arc<Mutex<Vec<String>>>,
}
impl ScopeConfigurationPolicy for DebugPolicy {
    fn configure(&self, _scope: &str) -> ScopeConfiguration {
        ScopeConfiguration {
            output: Box::new(DebugOutput {
                records: self.output.clone(),
            }),
            fmt: Box::new(NoColorFormatter),
        }
    }
}

#[test]
#[serial_test::serial]
fn scope_recognition() {
    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with(|| {
        let mut sim = AsyncBuilder::new();
        sim.node("node-a", |_| async {
            tracing::info!("node-a 0s #1");
            tracing::error!("node-a 0s #2");
            Ok(())
        });

        sim.node_with_parent("node-b", "node-a", |_| async {
            tracing::trace!("node-b 0s #1");
            Ok(())
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            [
                "[  0ns  ] INFO node-a (tracing): node-a 0s #1\n".to_string(),
                "[  0ns  ] ERROR node-a (tracing): node-a 0s #2\n".to_string(),
                "[  0ns  ] TRACE node-a.node-b (tracing): node-b 0s #1\n".to_string(),
            ]
        );
    });
}

#[test]
#[serial_test::serial]
fn time_recognition() {
    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with(|| {
        let mut sim = AsyncBuilder::new();
        sim.node("node-a", |_| async {
            tracing::info!("node-a 0s #1");
            sleep(Duration::from_secs(5)).await;
            tracing::error!("node-a 5s #2");
            Ok(())
        });
        sim.node_with_parent("node-b", "node-a", |_| async {
            tracing::trace!("node-b 0s #1");
            Ok(())
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            [
                "[  0ns  ] INFO node-a (tracing): node-a 0s #1\n".to_string(),
                "[  0ns  ] TRACE node-a.node-b (tracing): node-b 0s #1\n".to_string(),
                "[  5s   ] ERROR node-a (tracing): node-a 5s #2\n".to_string(),
            ]
        );
    });
}

#[test]
#[serial_test::serial]
fn span_recognition() {
    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with(|| {
        let mut sim = AsyncBuilder::new();

        sim.node("node-a", |_| {
            let span = tracing::span!(Level::DEBUG, "my-span");
            {
                async {
                    tracing::info!("node-a 0s #1");
                    Ok(())
                }
                .instrument(span)
            }
        });
        sim.node_with_parent("node-b", "node-a", |_| async {
            tracing::trace!("node-b 0s #1");
            Ok(())
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            [
                "[  0ns  ] INFO node-a (tracing): my-span node-a 0s #1\n".to_string(),
                "[  0ns  ] TRACE node-a.node-b (tracing): node-b 0s #1\n".to_string(),
            ]
        );
    });
}

#[test]
#[serial_test::serial]
fn span_fields_recognition() {
    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with(|| {
        let mut sim = AsyncBuilder::new();

        sim.node("node-a", |_| {
            let span = tracing::span!(Level::DEBUG, "my-span", key = 42);
            {
                async {
                    tracing::info!("node-a 0s #1");
                    Ok(())
                }
                .instrument(span)
            }
        });
        sim.node_with_parent("node-b", "node-a", |_| async {
            tracing::trace!("node-b 0s #1");
            Ok(())
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            [
                "[  0ns  ] INFO node-a (tracing): my-span{key = 42} node-a 0s #1\n".to_string(),
                "[  0ns  ] TRACE node-a.node-b (tracing): node-b 0s #1\n".to_string(),
            ]
        );
    });
}

#[test]
#[serial_test::serial]
fn multi_span_recogition() {
    #[tracing::instrument]
    async fn say_hello() {
        tracing::info!("hello")
    }

    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with(|| {
        let mut sim = AsyncBuilder::new();

        sim.node("node-a", |_| {
            let span = tracing::span!(Level::DEBUG, "my-span", key = 42);

            async {
                tracing::info!("node-a 0s #1");
                say_hello().await;
                Ok(())
            }
            .instrument(span)
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            [
                "[  0ns  ] INFO node-a (tracing): my-span{key = 42} node-a 0s #1\n".to_string(),
                "[  0ns  ] INFO node-a (tracing): my-span{key = 42} say_hello hello\n".to_string(),
            ]
        );
    });
}

#[test]
#[serial_test::serial]
fn filter_fallback_rule() {
    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with_filter("warn")
    .with(|| {
        let mut sim = AsyncBuilder::new();

        sim.node("node-a", |_| {
            let span = tracing::span!(Level::DEBUG, "my-span", key = 42);
            async {
                tracing::info!("node-a 0s #1");
                tracing::warn!("node-a 0s #2");
                Ok(())
            }
            .instrument(span)
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            ["[  0ns  ] WARN node-a (tracing): my-span{key = 42} node-a 0s #2\n".to_string(),]
        );
    });
}

mod submodule {
    pub fn say_hello() {
        tracing::info!("hello")
    }
}

#[test]
#[serial_test::serial]
fn filter_target_rule() {
    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with_filter("tracing::submodule=warn")
    .with(|| {
        let mut sim = AsyncBuilder::new();

        sim.node("node-a", |_| {
            let span = tracing::span!(Level::DEBUG, "my-span", key = 42);
            async {
                tracing::info!("node-a 0s #1");
                submodule::say_hello();
                Ok(())
            }
            .instrument(span)
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            ["[  0ns  ] INFO node-a (tracing): my-span{key = 42} node-a 0s #1\n".to_string(),]
        );
    });
}

#[test]
#[serial_test::serial]
fn filter_span_rule() {
    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with_filter("tracing::submodule=warn")
    .with(|| {
        let mut sim = AsyncBuilder::new();

        sim.node("node-a", |_| {
            let span = tracing::span!(Level::DEBUG, "my-span", key = 42);
            async {
                tracing::info!("node-a 0s #1");
                submodule::say_hello();
                Ok(())
            }
            .instrument(span)
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            ["[  0ns  ] INFO node-a (tracing): my-span{key = 42} node-a 0s #1\n".to_string(),]
        );
    });
}

#[test]
#[serial_test::serial]
fn filter_mixed_rule() {
    let records = Arc::new(Mutex::new(Vec::new()));
    Subscriber::new(DebugPolicy {
        output: records.clone(),
    })
    .with_filter("warn,tracing::submodule[span-name]=trace")
    .with(|| {
        let mut sim = AsyncBuilder::new();

        sim.node("node-a", |_| async {
            tracing::info!("#1"); // NO
            tracing::info!(target = "target", "#2"); // NO

            async {
                tracing::info!("#3"); // NO
                submodule::say_hello();
            }
            .instrument(tracing::span!(Level::TRACE, "span-name"))
            .await;

            Ok(())
        });

        let _ = Builder::new().build(sim.build()).run();

        let records = records.lock();
        assert_eq!(
            *records,
            ["[  0ns  ] INFO node-a (tracing::submodule): span-name hello\n".to_string(),]
        );
    });
}
