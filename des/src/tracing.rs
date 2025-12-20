//! Alternative tracing impl
use crate::{net::module::try_current, prelude::SimTime};
use nu_ansi_term::{Color, Style};
use tracing::{Level, Subscriber, dispatcher};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    filter::Directive,
    fmt::{self, FormatEvent, FormatFields, FormattedFields, format::Writer},
    layer::SubscriberExt,
    registry::LookupSpan,
};

/// The log level that will be used if `RUST_LOG` is not defined.
pub const FALLBACK_LOG_LEVEL: Level = Level::TRACE;

/// Create a new tracing subscriber with a sim formatter.
///
/// # Panics
///
/// Panics when subscriber initilization fails.
pub fn init() {
    let filter = EnvFilter::builder()
        .with_default_directive(Directive::from(FALLBACK_LOG_LEVEL))
        .from_env_lossy();

    let fmt_layer = fmt::layer().event_format(format()).with_filter(filter);

    let reg = Registry::default()
        .with(fmt_layer)
        .with(ErrorLayer::default());

    dispatcher::set_global_default(reg.into()).expect("failed to set global default subscriber");
}

/// An instance of a simulation formatter.
#[must_use]
pub fn format() -> SimFormat {
    SimFormat::init()
}

/// A formatter that includes simulation specific information into the tracing messages.
#[derive(Debug)]
pub struct SimFormat;

unsafe impl Sync for SimFormat {}

impl SimFormat {
    fn init() -> SimFormat {
        SimFormat
    }
}

macro_rules! maybe_ansi {
    ($style:ident, $ansi:ident, $writer:ident: $($t:tt)*) => {
        MaybeAnsi(format!($($t)*), $style, $ansi).write(&mut $writer)
    };
}

impl<S, N> FormatEvent<S, N> for SimFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();
        let ansi = writer.has_ansi_escapes();

        let dimmed = Style::new().dimmed();
        let bold = Style::new().bold();

        maybe_ansi!(dimmed, ansi, writer: "[ {:?} ] ", SimTime::now())?;

        let style = match *meta.level() {
            Level::TRACE => Style::new().fg(Color::Cyan),
            Level::DEBUG => Style::new().fg(Color::Purple),
            Level::INFO => Style::new().fg(Color::Green),
            Level::WARN => Style::new().fg(Color::Yellow),
            Level::ERROR => Style::new().fg(Color::Red),
        };

        if let Some(scope) = try_current().map(|v| v.path()) {
            if !ansi {
                write!(writer, "{} ", meta.level().as_str())?;
            }
            maybe_ansi!(style, ansi, writer: "{} ", scope)?;
        } else {
            maybe_ansi!(style, ansi, writer: "{} ", meta.level().as_str())?;
        }

        maybe_ansi!(dimmed, ansi, writer: "{}: ", meta.target())?;

        if let Some(scope) = ctx.event_scope() {
            let mut seen = false;
            for span in scope.from_root() {
                maybe_ansi!(bold, ansi, writer: "{}", span.metadata().name())?;
                seen = true;
                let ext = span.extensions();
                if let Some(fields) = &ext.get::<FormattedFields<N>>()
                    && !fields.is_empty()
                {
                    maybe_ansi!(bold, ansi, writer: "{{")?;
                    write!(writer, "{fields}")?;
                    maybe_ansi!(bold, ansi, writer: "}}")?;
                }

                maybe_ansi!(dimmed, ansi, writer: ":")?;
            }

            if seen {
                writer.write_char(' ')?;
            }
        }

        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

struct MaybeAnsi(String, Style, bool);

impl MaybeAnsi {
    fn write(self, writer: &mut Writer<'_>) -> std::fmt::Result {
        if self.2 {
            write!(writer, "{}", self.1.prefix())?;
            write!(writer, "{}", self.0)?;
            write!(writer, "{}", self.1.suffix())
        } else {
            write!(writer, "{}", self.0)
        }
    }
}
