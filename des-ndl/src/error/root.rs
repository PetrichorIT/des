use std::{
    error, fmt,
    io::{self, Write},
};

use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use super::{Error, ErrorHint, Errors};
use crate::resource::SourceMap;

pub type RootResult<T> = Result<T, RootError>;

pub struct RootError {
    pub errors: Errors,
    pub smap: SourceMap,
}

impl RootError {
    pub fn new(errors: Errors, smap: SourceMap) -> RootError {
        Self { errors, smap }
    }

    pub fn single(error: Error, smap: SourceMap) -> RootError {
        let mut errors = Errors::new();
        errors.list.push_back(error);
        Self { errors, smap }
    }
}

impl Error {
    fn fmt(&self, smap: &SourceMap, fmt: &mut Buffer) -> fmt::Result {
        self.fmt_title(fmt).map_err(|_| fmt::Error)?;
        self.fmt_span(smap, fmt).map_err(|_| fmt::Error)?;
        self.fmt_hint(smap, fmt).map_err(|_| fmt::Error)?;
        Ok(())
    }

    fn fmt_title(&self, fmt: &mut Buffer) -> io::Result<()> {
        fmt.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        write!(fmt, "error[{}]: ", self.kind as u8)?;

        fmt.reset()?;
        fmt.set_color(ColorSpec::new().set_bold(true))?;
        writeln!(fmt, "{} ({})", self.internal, self.kind)?;

        Ok(())
    }

    fn fmt_span(&self, smap: &SourceMap, fmt: &mut Buffer) -> io::Result<()> {
        let Some(span) = self.span else { return Ok(()) };
        let asset = smap.asset_for(span).expect("Failed to fetch asset");
        let line = smap.line_for(span).expect("Failed to fetch asset");

        // File path line
        fmt.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
        write!(fmt, "   --> ")?;

        fmt.reset()?;
        writeln!(
            fmt,
            "{}:{}",
            asset.ident.path().unwrap().to_str().unwrap(),
            line
        )?;

        let (pstr, p) = smap.slice_padded_for(span);
        // Print padded lines
        fmt.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
        if p == 0 {
            write!(fmt, "{:>3} | ", line)?;
        } else {
            write!(fmt, "    | ")?;
        }
        fmt.reset()?;

        let mut line_drawn = 1;

        for c in pstr.chars() {
            write!(fmt, "{}", c)?;
            if c == '\n' {
                fmt.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;

                if line_drawn != p {
                    write!(fmt, "    | ")?;
                } else {
                    write!(fmt, "{:>3} | ", line)?;
                }
                line_drawn += 1;
                fmt.reset()?;
            }
        }

        writeln!(fmt)?;

        Ok(())
    }

    fn fmt_hint(&self, smap: &SourceMap, fmt: &mut Buffer) -> io::Result<()> {
        for hint in &self.hints {
            match hint {
                ErrorHint::Help(help) => {
                    fmt.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                    write!(fmt, "    = ")?;

                    fmt.reset()?;
                    fmt.set_color(ColorSpec::new().set_bold(true))?;
                    write!(fmt, "help: ")?;

                    fmt.reset()?;
                    writeln!(fmt, "{}", help)?;
                }
                ErrorHint::Note(note) => {
                    fmt.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                    write!(fmt, "    = ")?;

                    fmt.reset()?;
                    fmt.set_color(ColorSpec::new().set_bold(true))?;
                    write!(fmt, "note: ")?;

                    fmt.reset()?;
                    writeln!(fmt, "{}", note)?;
                }
                ErrorHint::Solution(solution) => {
                    let sasset = smap.asset_for(solution.span).unwrap();
                    let sline = sasset.line_for(solution.span.pos);

                    fmt.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                    writeln!(fmt, "    = {}", solution.description)?;
                    writeln!(
                        fmt,
                        "       in {}:{}",
                        sasset.ident.path().unwrap().to_str().unwrap(),
                        sline
                    )?;

                    fmt.reset()?;
                }
            }
        }
        Ok(())
    }
}

impl error::Error for RootError {}
impl fmt::Display for RootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stream = BufferWriter::stderr(ColorChoice::Always);
        let mut buffer = stream.buffer();
        for error in &*self.errors {
            error.fmt(&self.smap, &mut buffer)?;
        }
        write!(f, "{}", String::from_utf8_lossy(buffer.as_slice()))
    }
}
impl fmt::Debug for RootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}
