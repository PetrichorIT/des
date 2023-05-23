#![allow(missing_docs, missing_debug_implementations, unreachable_pub)]

mod filter;
mod format;
mod output;
mod policy;

use self::{
    filter::TargetFilters,
    output::TracingRecord,
    policy::{DefaultScopeConfigurationPolicy, ScopeConfigurationPolicy},
};
use crate::{prelude::SimTime, sync::RwLock};
use std::{collections::HashMap, sync::atomic::AtomicU64};
use termcolor::BufferWriter;
use tracing::{
    field::Visit, level_filters::STATIC_MAX_LEVEL, metadata::LevelFilter, span,
    subscriber::SetGlobalDefaultError,
};

pub use self::format::TracingFormatter;
pub use self::output::TracingOutput;

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
    scopes: RwLock<HashMap<u64, Scope>>,
    filters: TargetFilters,
    max_log_level: LevelFilter,

    //
    span_id: AtomicU64,
    spans: RwLock<HashMap<span::Id, SpanInfo>>,
    stack: RwLock<Vec<span::Id>>,
    active_scope: AtomicU64,
}

struct Scope {
    path: String,
    output: Box<dyn TracingOutput>,
    fmt: Box<dyn TracingFormatter>,
}

struct SpanInfo {
    pub formatted: String,
}

const SPAN_ID_MASK: u64 = 0x80_00_00_00_00_00_00_00;

impl<P: ScopeConfigurationPolicy> Subscriber<P> {
    pub fn new(policy: P) -> Self {
        Self {
            policy,
            scopes: RwLock::new(HashMap::new()),
            filters: TargetFilters::new(true),
            max_log_level: STATIC_MAX_LEVEL,

            span_id: AtomicU64::new(1),
            spans: RwLock::new(HashMap::new()),
            stack: RwLock::new(Vec::new()),
            active_scope: AtomicU64::new(0),
        }
    }

    pub fn with_max_level(mut self, level: LevelFilter) -> Self {
        self.max_log_level = level;
        self
    }

    pub fn with_filter(mut self, filter: impl AsRef<str>) -> Self {
        self.filters.parse_str(filter.as_ref());
        self
    }

    pub fn init(self) -> Result<(), SetGlobalDefaultError>
    where
        P: 'static,
    {
        tracing::subscriber::set_global_default(self)
    }
}

impl<P: ScopeConfigurationPolicy> Subscriber<P> {
    fn register_new_scope(&self, span: &span::Attributes<'_>) -> span::Id {
        let id = self
            .span_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            | SPAN_ID_MASK;

        struct PathExtractor<'a> {
            path: &'a mut String,
        }
        impl Visit for PathExtractor<'_> {
            fn record_debug(
                &mut self,
                _field: &tracing::field::Field,
                value: &dyn std::fmt::Debug,
            ) {
                use std::fmt::Write;
                write!(self.path, "{:?}", value).unwrap();
            }
        }

        let mut path = String::new();
        span.values().record(&mut PathExtractor { path: &mut path });
        let path = path.trim_matches('"').to_string();
        let path = if path.is_empty() {
            String::from("@root")
        } else {
            path
        };

        let cfg = self.policy.configure(&path);
        self.scopes.write().insert(
            id.clone(),
            Scope {
                output: cfg.output,
                fmt: cfg.fmt,
                path,
            },
        );

        span::Id::from_u64(id)
    }
}

impl<P: ScopeConfigurationPolicy + 'static> tracing::Subscriber for Subscriber<P> {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        self.max_log_level >= *metadata.level()
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        if span.metadata().name() == "--des-module" {
            self.register_new_scope(span)
        } else {
            let id = span::Id::from_u64(
                self.span_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            );
            let scope_id = self.active_scope.load(std::sync::atomic::Ordering::SeqCst);
            let mut scopes = self.scopes.write();
            let scope = scopes.get_mut(&scope_id).unwrap();

            let info = SpanInfo::from_attrs(span, &mut *scope.fmt);
            self.spans.write().insert(id.clone(), info);
            id
        }
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
        let scope_id = self.active_scope.load(std::sync::atomic::Ordering::SeqCst);
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
            todo!()
        }
    }

    fn enter(&self, span: &span::Id) {
        let is_scope = span.into_u64() & SPAN_ID_MASK != 0;
        if is_scope {
            self.active_scope
                .store(span.into_u64(), std::sync::atomic::Ordering::SeqCst);
        } else {
            self.stack.write().push(span.clone());
        }
    }

    fn exit(&self, span: &span::Id) {
        let is_scope = span.into_u64() & SPAN_ID_MASK != 0;
        if is_scope {
            self.active_scope
                .store(0, std::sync::atomic::Ordering::SeqCst);
        } else {
            assert_eq!(self.stack.write().pop(), Some(span.clone()))
        }
    }

    fn try_close(&self, id: span::Id) -> bool {
        // 0
        let is_scope = id.into_u64() & SPAN_ID_MASK != 0;
        if !is_scope {
            self.scopes.write().remove(&id.into_u64());
        }
        false
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
            // level: *attr.metadata().level(),
        }
    }
}

unsafe impl<P: ScopeConfigurationPolicy> Send for Subscriber<P> {}
unsafe impl<P: ScopeConfigurationPolicy> Sync for Subscriber<P> {}
