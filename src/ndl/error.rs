use std::{io::Write, mem::MaybeUninit};
use termcolor::*;

use super::{
    loc::Loc,
    souce::{SourceAsset, SourceAssetDescriptor},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalErrorContext {
    pub lexing_errors: Vec<Error>,
    pub parsing_errors: Vec<Error>,
    pub tychecking_errors: Vec<Error>,
}

impl GlobalErrorContext {
    pub fn new() -> Self {
        Self {
            lexing_errors: Vec::new(),
            parsing_errors: Vec::new(),
            tychecking_errors: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !(self.lexing_errors.is_empty()
            && self.parsing_errors.is_empty()
            && self.tychecking_errors.is_empty())
    }

    pub fn can_parse(&self) -> bool {
        !self
            .lexing_errors
            .iter()
            .any(|e| e.code == ErrorCode::LexInvalidSouceIdentifier)
    }

    pub fn can_tycheck(&self) -> bool {
        // TODO
        true
    }
}

impl Default for GlobalErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalParsingErrorContext<'a> {
    pub errors: Vec<Error>,

    asset: &'a SourceAsset,
    transient: bool,
}

impl IntoIterator for LocalParsingErrorContext<'_> {
    type Item = Error;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a> LocalParsingErrorContext<'a> {
    pub fn null() -> Self {
        Self {
            errors: Vec::new(),

            asset: unsafe { &*MaybeUninit::<SourceAsset>::uninit().as_ptr() },
            transient: false,
        }
    }

    pub fn new(asset: &'a SourceAsset) -> Self {
        Self {
            errors: Vec::new(),

            asset,
            transient: false,
        }
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn record(&mut self, code: ErrorCode, msg: String, loc: Loc) {
        self.errors.push(Error {
            code,
            msg,
            source: loc.padded_referenced_slice_in(&self.asset.data).to_string(),

            loc,
            asset: self.asset.descriptor.clone(),

            transient: self.transient,
        });
        self.transient = true;
    }

    pub fn reset_transient(&mut self) {
        self.transient = false;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: ErrorCode,
    pub msg: String,
    pub source: String,

    pub asset: SourceAssetDescriptor,
    pub loc: Loc,

    pub transient: bool,
}

impl Error {
    pub fn new(
        code: ErrorCode,
        msg: String,
        loc: Loc,
        transient: bool,
        asset: &SourceAsset,
    ) -> Self {
        let source = loc.padded_referenced_slice_in(&asset.data).to_string();
        let asset = asset.descriptor.clone();

        Self {
            code,
            msg,
            source,

            asset,
            loc,

            transient,
        }
    }

    pub fn print(&self) -> std::io::Result<()> {
        let mut stream = StandardStream::stderr(ColorChoice::Always);

        stream.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        if self.transient {
            write!(&mut stream, "*error[{}]: ", self.code as i32)?;
        } else {
            write!(&mut stream, " error[{}]: ", self.code as i32)?;
        }

        stream.reset()?;
        stream.set_color(ColorSpec::new().set_bold(true))?;
        writeln!(&mut stream, "{}", self.msg)?;

        stream.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
        write!(&mut stream, "   --> ")?;

        stream.reset()?;
        writeln!(
            &mut stream,
            "{}:{}",
            self.asset.path.to_str().unwrap(),
            self.loc.line
        )?;

        stream.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
        write!(&mut stream, "    |")?;
        stream.reset()?;

        let mut line_drawn = false;

        for c in self.source.chars() {
            write!(&mut stream, "{}", c)?;
            if c == '\n' {
                stream.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;

                if line_drawn {
                    write!(&mut stream, "    |")?;
                } else {
                    write!(&mut stream, "{:>3} |", self.loc.line)?;
                    line_drawn = true
                }
                stream.reset()?;
            }
        }

        writeln!(&mut stream)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ParLinkMissingIdentifier = 1,
    ParLinkMissingDefBlockOpen = 2,
    ParLinkMissingDefBlockClose = 3,
    ParLinkInvalidKeyToken = 4,
    ParLinkInvalidKey = 5,
    ParLinkInvalidKvSeperator = 6,
    ParLinkInvalidValueToken = 7,
    ParLinkInvalidValueType = 8,

    ParModuleMissingIdentifer = 20,
    ParModuleMissingDefBlockOpen = 21,
    ParModuleMissingSectionIdentifier = 23,
    ParModuleInvalidSectionIdentifer = 24,
    ParModuleInvalidSeperator = 25,
    ParModuleInvalidKeyToken = 26,
    ParModuleGateMissingClosingBracket = 27,
    ParModuleGateInvalidIdentifierToken = 28,
    ParModuleGateInvalidGateSize = 29,
    ParModuleSubInvalidIdentiferToken = 30,
    ParModuleSubInvalidSeperator = 31,
    ParModuleConInvalidIdentiferToken = 32,
    ParModuleConInvaldiChannelSyntax = 33,

    LiteralIntParseError = 100,
    LiteralFloatParseError = 101,

    LexInvalidSouceToken = 201,
    LexInvalidSouceIdentifier = 202,

    TycDefNameCollission = 300,
    TycModuleSubmoduleFieldAlreadyDeclared = 301,
    TycModuleSubmoduleRecrusiveTyDefinition = 302,
    TycModuleSubmoduleInvalidTy = 303,
    TycModuleConInvalidChannelTy = 304,
    TycModuleConUnknownIdentSymbol = 305,
}
