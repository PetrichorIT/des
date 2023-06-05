use std::io::{Result, Write};
use termcolor::{Buffer, Color, ColorSpec, WriteColor};
use tracing::{field::Visit, span::Attributes, Event, Level};

use crate::prelude::SimTime;

use super::output::TracingRecord;

/// A formatter for a tracing subscriber scope.
pub trait TracingFormatter {
    /// Formats an emitted tracing event onto a buffer.
    fn fmt(&mut self, out: &mut Buffer, record: TracingRecord<'_>) -> Result<()>;

    /// Formats a new span for later use
    fn fmt_new_span(&mut self, out: &mut Buffer, span: &Attributes<'_>) -> Result<()>;
}

/// A formatter intenden for a ANIS terminal.
#[derive(Debug)]
pub struct ColorfulTracingFormatter;
impl TracingFormatter for ColorfulTracingFormatter {
    fn fmt(&mut self, out: &mut Buffer, record: TracingRecord<'_>) -> Result<()> {
        self.fmt_time(out, record.time)?;
        write!(out, " ")?;
        self.fmt_scope(
            out,
            record.scope,
            record.target,
            *record.event.metadata().level(),
        )?;
        write!(out, " ")?;
        self.fmt_spans(out, &record.spans)?;
        self.fmt_event(out, record.event)?;
        writeln!(out)
    }

    fn fmt_new_span(&mut self, out: &mut Buffer, span: &Attributes<'_>) -> Result<()> {
        struct Vis<'a> {
            s: &'a mut String,
        }
        impl Visit for Vis<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                write!(self.s, "{}={:?}, ", field.name(), value).unwrap();
            }
        }

        if span.values().is_empty() {
            out.set_color(ColorSpec::new().set_bold(true))?;
            write!(out, "{}", span.metadata().name())?;
            out.reset()
        } else {
            out.set_color(ColorSpec::new().set_bold(true))?;
            write!(out, "{}", span.metadata().name())?;
            out.set_color(ColorSpec::new().set_bold(false).set_fg(Some(PARENS_COLOR)))?;

            let mut s = String::new();
            span.values().record(&mut Vis { s: &mut s });
            write!(out, "{{ {} }}", s.trim_end_matches(", "))?;
            out.reset()
        }
    }
}

impl ColorfulTracingFormatter {
    fn fmt_time(&mut self, out: &mut Buffer, time: SimTime) -> Result<()> {
        out.set_color(ColorSpec::new().set_fg(Some(PARENS_COLOR)))?;
        write!(out, "[ ")?;
        let time_str = format!("{}", time);
        write!(out, "{time_str:^5}")?;
        write!(out, " ]")?;
        out.reset()
    }

    fn fmt_scope(
        &mut self,
        out: &mut Buffer,
        scope: Option<&str>,
        target: Option<&str>,
        level: Level,
    ) -> Result<()> {
        let color = get_level_color(level);
        if let Some(scope) = scope {
            out.set_color(ColorSpec::new().set_fg(Some(color)))?;
            write!(out, "{}", scope)?;
        }

        out.set_color(ColorSpec::new().set_fg(Some(color)).set_bold(true))?;
        if let Some(target) = target {
            write!(out, " ({target})")?;
        }

        write!(out, ":")?;
        out.reset()
    }

    fn fmt_spans(&mut self, out: &mut Buffer, spans: &[&str]) -> Result<()> {
        out.set_color(ColorSpec::new().set_bold(true))?;
        let end = spans.len();
        for (i, span) in spans.into_iter().enumerate() {
            write!(out, "{}", span)?;
            if i + 1 < end {
                write!(out, ":")?;
            } else {
                write!(out, " ")?;
            }
        }
        out.reset()
    }

    fn fmt_event(&mut self, out: &mut Buffer, event: &Event<'_>) -> Result<()> {
        struct Vis<'a> {
            values: &'a mut String,
            message: &'a mut String,
        }
        impl Visit for Vis<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                if field.name() == "message" {
                    write!(self.message, "{:?}", value).unwrap();
                } else {
                    write!(self.values, "{} = {:?}", field.name(), value).unwrap();
                }
            }
        }

        let mut values = String::new();
        let mut message = String::new();
        event.record(&mut Vis {
            values: &mut values,
            message: &mut message,
        });
        if values.is_empty() {
            write!(out, "{}", message)
        } else {
            write!(out, "{} {}", values, message)
        }
    }
}

const PARENS_COLOR: Color = Color::Rgb(0x7f, 0x8c, 0x8d);
const fn get_level_color(level: Level) -> Color {
    match level {
        Level::DEBUG => Color::Magenta,
        Level::TRACE => Color::Cyan,
        Level::INFO => Color::Green,
        Level::WARN => Color::Yellow,
        Level::ERROR => Color::Red,
    }
}

/// A formatter for raw strings.
#[derive(Debug)]
pub struct NoColorFormatter;
impl TracingFormatter for NoColorFormatter {
    fn fmt(&mut self, out: &mut Buffer, record: TracingRecord<'_>) -> Result<()> {
        write!(out, "[ ")?;
        let time_str = format!("{}", record.time);
        write!(out, "{time_str:^5} ] ")?;

        match *record.event.metadata().level() {
            Level::ERROR => write!(out, "ERROR "),
            Level::WARN => write!(out, "WARN "),
            Level::INFO => write!(out, "INFO "),
            Level::DEBUG => write!(out, "DEBUG "),
            Level::TRACE => write!(out, "TRACE "),
        }?;

        if let Some(scope) = record.scope {
            write!(out, "{}", scope)?;
        }
        if let Some(target) = record.target {
            write!(out, " ({target})")?;
        }
        write!(out, ": ")?;

        for span in record.spans {
            write!(out, "{} ", span)?;
        }

        struct Vis<'a> {
            values: &'a mut String,
            message: &'a mut String,
        }
        impl Visit for Vis<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                if field.name() == "message" {
                    write!(self.message, "{:?}", value).unwrap();
                } else {
                    write!(self.values, "{} = {:?}", field.name(), value).unwrap();
                }
            }
        }

        let mut values = String::new();
        let mut message = String::new();
        record.event.record(&mut Vis {
            values: &mut values,
            message: &mut message,
        });
        if values.is_empty() {
            write!(out, "{}", message)?;
        } else {
            write!(out, "{} {}", values, message)?;
        }

        writeln!(out)
    }

    fn fmt_new_span(&mut self, out: &mut Buffer, span: &Attributes<'_>) -> Result<()> {
        struct Vis<'a> {
            s: &'a mut String,
        }
        impl Visit for Vis<'_> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                write!(self.s, "{} = {:?},", field.name(), value).unwrap();
            }
        }

        if span.values().is_empty() {
            write!(out, "{}", span.metadata().name())
        } else {
            write!(out, "{}", span.metadata().name())?;
            let mut s = String::new();
            span.values().record(&mut Vis { s: &mut s });
            write!(out, "{{{}}}", s.trim_end_matches(','))
        }
    }
}
