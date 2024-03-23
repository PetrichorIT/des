//! Alternative tracing impl

use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender},
};

use crate::{
    prelude::{ObjectPath, SimTime},
    sync::{Mutex, RwLock},
};
use fxhash::{FxBuildHasher, FxHashMap};
use nu_ansi_term::{Color, Style};
use tracing::{Level, Subscriber};
use tracing_subscriber::{
    filter::Directive,
    fmt::{format::Writer, FormatEvent, FormatFields, FormattedFields},
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter,
};

/// A token describing a logger scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeToken(u64);

static SCOPE_CURRENT_TOKEN: AtomicU64 = AtomicU64::new(u64::MAX);
static SCOPE_TOKEN_NEXT: AtomicU64 = AtomicU64::new(0);
static SCOPES: Mutex<Option<Sender<(ScopeToken, ObjectPath)>>> = Mutex::new(None);

/// Creates a new scope attached to the tracing subscriber.
///
/// This function is intended for internal use, but remains
/// public, since it may be usefull in rare scenarios
#[doc(hidden)]
pub fn new_scope(obj_path: ObjectPath) -> ScopeToken {
    let token = ScopeToken(SCOPE_TOKEN_NEXT.fetch_add(1, Ordering::SeqCst));
    let lock = SCOPES.lock();
    if let Some(scopes) = &*lock {
        scopes.send((token, obj_path)).expect("Failed to send");
    } else {
        // WARNING MAYBE
    }
    token
}

/// Indicates that the begin of a scope, that was allread registerd.
///
/// This function is intended for internal use, but remains
/// public, since it may be usefull in rare scenarios
#[doc(hidden)]
pub fn enter_scope(token: ScopeToken) {
    SCOPE_CURRENT_TOKEN.store(token.0, Ordering::SeqCst);
}

/// Indicates that no scope is currently active.
///
/// This function is intended for internal use, but remains
/// public, since it may be usefull in rare scenarios
#[doc(hidden)]
pub fn leave_scope() {
    SCOPE_CURRENT_TOKEN.store(u64::MAX, Ordering::SeqCst);
}

/// The log level that will be used if `RUST_LOG` is not defined.
pub const FALLBACK_LOG_LEVEL: Level = Level::TRACE;

/// Create a new tracing subscriber with a sim formatter.
///
/// # Panics
///
/// Panics when subscriber initilization fails.
pub fn init() {
    let subscriber = tracing_subscriber::fmt();
    let subscriber = subscriber.event_format(format());
    let subscriber = subscriber.with_env_filter(
        EnvFilter::builder()
            .with_default_directive(Directive::from(FALLBACK_LOG_LEVEL))
            .from_env_lossy(),
    );
    subscriber.finish().init();
}

/// An instance of a simulation formatter.
#[must_use]
pub fn format() -> SimFormat {
    SimFormat::init()
}

/// A formatter that includes simulation specific information into the tracing messages.
#[derive(Debug)]
pub struct SimFormat {
    scopes: RwLock<FxHashMap<u64, Scope>>,
    rx: Mutex<Receiver<(ScopeToken, ObjectPath)>>,
}

unsafe impl Sync for SimFormat {}

#[derive(Debug)]
struct Scope {
    path: ObjectPath,
}

impl SimFormat {
    fn init() -> SimFormat {
        let (tx, rx) = mpsc::channel();
        SCOPES.lock().replace(tx);
        SimFormat {
            scopes: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),
            rx: Mutex::new(rx),
        }
    }

    fn fetch_scopes(&self) {
        let rx = self.rx.lock();
        let mut scopes = self.scopes.write();
        while let Ok((new_token, new_scope)) = rx.try_recv() {
            scopes.insert(new_token.0, Scope { path: new_scope });
        }
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

        self.fetch_scopes();
        let scope_id = SCOPE_CURRENT_TOKEN.load(Ordering::SeqCst);
        let scopes = self.scopes.read();

        if let Some(scope) = scopes.get(&scope_id) {
            if !ansi {
                write!(writer, "{} ", meta.level().as_str())?;
            }
            maybe_ansi!(style, ansi, writer: "{} ", scope.path)?;
        } else {
            maybe_ansi!(style, ansi, writer: "{} ", meta.level().as_str())?;
        }

        if let Some(scope) = ctx.event_scope() {
            let mut seen = false;
            for span in scope.from_root() {
                maybe_ansi!(bold, ansi, writer: "{}", span.metadata().name())?;
                seen = true;
                let ext = span.extensions();
                if let Some(fields) = &ext.get::<FormattedFields<N>>() {
                    if !fields.is_empty() {
                        maybe_ansi!(bold, ansi, writer: "{{")?;
                        write!(writer, "{fields}")?;
                        maybe_ansi!(bold, ansi, writer: "}}")?;
                    }
                }
                maybe_ansi!(dimmed, ansi, writer: ":")?;
            }

            if seen {
                writer.write_char(' ')?;
            }
        }

        maybe_ansi!(dimmed, ansi, writer: "{}: ", meta.target())?;

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
