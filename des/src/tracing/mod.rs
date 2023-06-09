//! Structured event tracing with custom context

mod filter;
mod format;
mod output;
mod policy;

use self::{filter::TargetFilters, policy::DefaultScopeConfigurationPolicy};
use crate::{
    prelude::SimTime,
    sync::{Mutex, RwLock},
};
use fxhash::{FxBuildHasher, FxHashMap};
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{channel, Receiver, Sender},
    },
};
use termcolor::BufferWriter;
use tracing::{
    level_filters::STATIC_MAX_LEVEL, metadata::LevelFilter, span, subscriber::SetGlobalDefaultError,
};

pub use self::format::ColorfulTracingFormatter;
pub use self::format::NoColorFormatter;
pub use self::format::TracingFormatter;
pub use self::output::TracingOutput;
pub use self::output::TracingRecord;
pub use self::policy::ScopeConfiguration;
pub use self::policy::ScopeConfigurationPolicy;

/// A token describing a logger scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeToken(u64);

// is only set on the simulation thread, but read by all
// use static mut with a file-local saftey contract.
static SCOPE_CURRENT_TOKEN: AtomicU64 = AtomicU64::new(u64::MAX);
static SCOPE_TOKEN_NEXT: AtomicU64 = AtomicU64::new(0);
static SCOPES: Mutex<Option<Sender<(ScopeToken, String)>>> = Mutex::new(None);

/// Creates a new scope attached to the tracing subscriber.
///
/// This function is intended for internal use, but remains
/// public, since it may be usefull in rare scenarios
#[doc(hidden)]
pub fn new_scope(s: &str) -> ScopeToken {
    let token = ScopeToken(SCOPE_TOKEN_NEXT.fetch_add(1, Ordering::SeqCst));
    let lock = SCOPES.lock();
    if let Some(scopes) = &*lock {
        scopes.send((token, s.to_string())).expect("Failed to send");
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

/// A subscriber to tracing events emitted by `des`.
///
/// Note that this subscriber extends the usual functionality of
/// tracing, by capturing the module context, aswell as the simulation
/// time too. Capturing the simulation context is
/// done by introducing custom spans.
///
/// This subscriber should only be used in combination with a
/// des `Runtime` that executes a `NetworkApplication`.
pub struct Subscriber<P: ScopeConfigurationPolicy> {
    policy: P,
    scopes: RwLock<FxHashMap<u64, Scope>>,
    filters: TargetFilters,
    max_log_level: LevelFilter,

    scopes_tx: Sender<(ScopeToken, String)>,
    scopes_rx: Mutex<Receiver<(ScopeToken, String)>>,

    //
    span_id: AtomicU64,
    spans: RwLock<FxHashMap<span::Id, SpanInfo>>,
    stack: RwLock<Vec<span::Id>>,
}

struct Scope {
    path: String,
    output: Box<dyn TracingOutput>,
    fmt: Box<dyn TracingFormatter>,
}

struct SpanInfo {
    formatted: String,
    sc: usize,
}

impl<P: ScopeConfigurationPolicy> Subscriber<P> {
    /// Creates a new tracing Subscriber with the given policy.
    pub fn new(policy: P) -> Self {
        let (scopes_tx, scopes_rx) = channel();

        Self {
            policy,
            scopes: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),
            filters: TargetFilters::new(true),
            max_log_level: STATIC_MAX_LEVEL,

            scopes_tx,
            scopes_rx: Mutex::new(scopes_rx),

            span_id: AtomicU64::new(1),
            spans: RwLock::new(FxHashMap::with_hasher(FxBuildHasher::default())),
            stack: RwLock::new(Vec::new()),
        }
    }

    /// Sets the maximum log level of the subscriber.
    ///
    /// All trace events not at least reaching this level,
    /// will be discarded.
    pub fn with_max_level(mut self, level: LevelFilter) -> Self {
        self.max_log_level = level;
        self
    }

    /// Adds a target filter in textual repr to the subscriber.
    pub fn with_filter(mut self, filter: impl AsRef<str>) -> Self {
        self.filters.parse_str(filter.as_ref());
        self
    }

    /// Sets the tracer as the global default.
    pub fn init(self) -> Result<(), SetGlobalDefaultError>
    where
        P: 'static,
    {
        let tx = self.scopes_tx.clone();
        tracing::subscriber::set_global_default(self)?;
        let _ = SCOPES.lock().replace(tx);
        Ok(())
    }
}

impl<P: ScopeConfigurationPolicy> Subscriber<P> {
    fn check_scopes(&self) {
        let rx = self.scopes_rx.lock();
        while let Ok((id, scope_name)) = rx.try_recv() {
            let mut scopes = self.scopes.write();
            let cfg = self.policy.configure(&scope_name);
            let _a = scopes.insert(
                id.0,
                Scope {
                    path: if scope_name.is_empty() {
                        "@root".into()
                    } else {
                        scope_name
                    },
                    output: cfg.output,
                    fmt: cfg.fmt,
                },
            );
            assert!(_a.is_none());
        }
    }
}

impl<P: ScopeConfigurationPolicy + 'static> tracing::Subscriber for Subscriber<P> {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        self.max_log_level >= *metadata.level()
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        self.check_scopes();

        let id = span::Id::from_u64(
            self.span_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        );
        let scope_id = SCOPE_CURRENT_TOKEN.load(std::sync::atomic::Ordering::SeqCst);
        let mut scopes = self.scopes.write();

        let scope = scopes.get_mut(&scope_id).unwrap();

        let info = SpanInfo::from_attrs(span, &mut *scope.fmt);
        self.spans.write().insert(id.clone(), info);
        id
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {
        todo!()
    }

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {
        todo!()
    }

    fn event(&self, event: &tracing::Event<'_>) {
        // (-1) Target
        let target = if Some(event.metadata().target()) == event.metadata().module_path() {
            None
        } else {
            Some(event.metadata().target())
        };

        if let Some(target) = target {
            let allowed_max_level = self.filters.filter_for(target, LevelFilter::TRACE);
            if allowed_max_level < *event.metadata().level() {
                return;
            }
        }

        // (0) Identify current scope
        self.check_scopes();
        let scope_id = SCOPE_CURRENT_TOKEN.load(Ordering::SeqCst);
        let mut scopes = self.scopes.write();
        let scope = scopes.get_mut(&scope_id);

        // (1) Collect active spans
        let spans = self.spans.read();
        let active = self
            .stack
            .read()
            .iter()
            .map(|id| spans.get(id).unwrap().formatted.as_str())
            .collect::<Vec<_>>();

        let mut record = TracingRecord {
            time: SimTime::now(),
            scope: None,
            target,
            spans: &active,
            event,
        };

        if let Some(Scope { output, fmt, path }) = scope {
            // must be in this order because brwchk
            record.scope = Some(&*path);
            output.write(&mut **fmt, record).unwrap();
        } else {
            // TODO: todo!()
        }
    }

    fn enter(&self, span: &span::Id) {
        self.stack.write().push(span.clone());
    }

    fn exit(&self, span: &span::Id) {
        assert_eq!(self.stack.write().pop(), Some(span.clone()))
    }

    fn try_close(&self, id: span::Id) -> bool {
        let mut spans = self.spans.write();
        let Some(span) = spans.get_mut(&id) else {
            return false;
        };
        span.sc -= 1;
        if span.sc == 0 {
            spans.remove(&id);
        }

        false
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.spans.write().get_mut(id).map(|info| info.sc += 1);
        id.clone()
    }
}

impl Default for Subscriber<DefaultScopeConfigurationPolicy> {
    fn default() -> Self {
        Self::new(DefaultScopeConfigurationPolicy)
    }
}

impl SpanInfo {
    fn from_attrs(attr: &span::Attributes, fmt: &mut dyn TracingFormatter) -> Self {
        let output = BufferWriter::stdout(termcolor::ColorChoice::Never);
        let mut buffer = output.buffer();
        fmt.fmt_new_span(&mut buffer, attr).unwrap();

        Self {
            formatted: String::from_utf8_lossy(buffer.as_slice()).into_owned(),
            sc: 1,
        }
    }
}

unsafe impl<P: ScopeConfigurationPolicy> Send for Subscriber<P> {}
unsafe impl<P: ScopeConfigurationPolicy> Sync for Subscriber<P> {}

impl<P: ScopeConfigurationPolicy> Debug for Subscriber<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber").finish()
    }
}