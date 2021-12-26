use std::{io::Write, mem::MaybeUninit};
use termcolor::*;

use crate::{parser::ParResult, source::Asset, SourceMap, Token, TokenKind};

use super::loc::Loc;

///
/// A global context for storing errors, and if nessecary stopping
/// the next resolving steps.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalErrorContext {
    /// The errors that were found as invalid tokens in the token stream.
    pub lexing_errors: Vec<Error>,
    /// The syntatic errors found while parsing an asset.
    pub parsing_errors: Vec<Error>,
    /// The semantic errors found while evaluating the workspace.
    pub tychecking_errors: Vec<Error>,
}

impl GlobalErrorContext {
    ///
    /// Creates a new raw instace.
    ///
    pub fn new() -> Self {
        Self {
            lexing_errors: Vec::new(),
            parsing_errors: Vec::new(),
            tychecking_errors: Vec::new(),
        }
    }

    ///
    /// Indicates whether an error has occured.
    ///
    pub fn has_errors(&self) -> bool {
        !(self.lexing_errors.is_empty()
            && self.parsing_errors.is_empty()
            && self.tychecking_errors.is_empty())
    }

    ///
    /// Indicates whether the parsing step can be done.
    ///
    pub fn can_parse(&self) -> bool {
        !self
            .lexing_errors
            .iter()
            .any(|e| e.code == ErrorCode::LexInvalidSouceIdentifier)
    }

    ///
    /// Indicates whether typchecking can be done.
    ///
    pub fn can_tycheck(&self) -> bool {
        use ErrorCode::*;
        !self
            .parsing_errors
            .iter()
            .any(|e| matches!(e.code, ParUnexpectedEOF | TooManyErrors))
    }
}

impl Default for GlobalErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

///
/// A local error context for the lexcheck phase.
///
#[derive(Debug)]
pub struct LexingErrorContext<'a> {
    errors: Vec<Error>,
    asset: Asset<'a>,
}

impl<'a> LexingErrorContext<'a> {
    ///
    /// Creates a new lexing error context.
    ///
    pub fn new(asset: Asset<'a>) -> Self {
        Self {
            errors: Vec::new(),
            asset,
        }
    }

    /// The number of errors in this context.1
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    ///
    ///  An indicator whether too many errors have occurred.
    ///
    pub fn exceeded_error_limit(&self) -> bool {
        self.len() > self.asset.mapped_asset().len_lines + 5
    }

    ///
    /// A function to add another error to the context.
    ///
    pub fn record(&mut self, token: &Token) -> ParResult<()> {
        self.errors.push(Error::new_lex(
            if matches!(token.kind, TokenKind::InvalidIdent) {
                ErrorCode::LexInvalidSouceIdentifier
            } else {
                ErrorCode::LexInvalidSouceToken
            },
            token.loc,
            self.asset,
        ));

        if self.exceeded_error_limit() {
            Err("Too many lexing errors.")
        } else {
            Ok(())
        }
    }

    ///
    /// An extraction function that returns the collected errors or
    /// non if they exceed the error limit.
    ///
    pub fn finish(self) -> Vec<Error> {
        if self.exceeded_error_limit() {
            Vec::new()
        } else {
            self.errors
        }
    }
}

///
/// A local error context for creating transient errors
/// during the parsing stage.
///
#[derive(Debug, Clone)]
pub struct ParsingErrorContext<'a> {
    /// A list of the collect errors, including transients.
    pub errors: Vec<Error>,

    asset: &'a Asset<'a>,
    transient: bool,
}

impl IntoIterator for ParsingErrorContext<'_> {
    type Item = Error;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a> ParsingErrorContext<'a> {
    ///
    /// Creates an error context without asset binding.
    /// This cannot be usied without binding to an asset.
    ///
    pub fn null() -> Self {
        Self {
            errors: Vec::new(),

            asset: unsafe { &*MaybeUninit::<Asset>::uninit().as_ptr() },
            transient: false,
        }
    }

    ///
    /// Creates a new context bound to the given asset.
    ///
    pub fn new(asset: &'a Asset) -> Self {
        Self {
            errors: Vec::new(),

            asset,
            transient: false,
        }
    }

    ///
    /// The number of errors record.
    ///
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    ///
    /// Imdicates whether any errors have occured.
    ///
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    ///
    /// Records an error, determining transients automaticly.
    ///
    pub fn record(&mut self, code: ErrorCode, msg: String, loc: Loc) -> ParResult<()> {
        self.errors.push(Error {
            code,
            msg,

            solution: None,

            loc,

            transient: self.transient,
        });
        self.transient = true;

        if self.errors.len() > self.asset.mapped_asset().len_lines + 5 {
            Err("Too many errors")
        } else {
            Ok(())
        }
    }

    ///
    /// Records an error with solution, determining transients automaticly.
    ///
    pub fn record_with_solution(
        &mut self,
        code: ErrorCode,
        msg: String,
        loc: Loc,
        solution: ErrorSolution,
    ) -> ParResult<()> {
        self.errors.push(Error {
            code,
            msg,
            solution: Some(solution),

            loc,

            transient: self.transient,
        });
        self.transient = true;

        if self.errors.len() > self.asset.mapped_asset().len_lines + 5 {
            Err("Too many errors")
        } else {
            Ok(())
        }
    }

    ///
    /// Resets the transient flag.
    ///
    pub fn reset_transient(&mut self) {
        self.transient = false;
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ErrorSolution {
    pub msg: String,

    pub loc: Loc,
    // ### Autofix properties ###
    // pub insert_loc: Loc
    // pub insert_ele: String
    //
    // pub remove_loc: Loc,
}

impl ErrorSolution {
    ///
    /// Creates a new ErrorSolution.
    ///
    pub fn new(msg: String, loc: Loc) -> Self {
        Self { msg, loc }
    }
}

///
/// A generic NDL error.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// An unquie identifer for the error class.
    pub code: ErrorCode,
    /// An instance-specific error message.
    pub msg: String,
    /// A possible solution for the error.
    pub solution: Option<ErrorSolution>,

    /// The exact location of the error.
    pub loc: Loc,

    /// A indicator whether this error syntacticly was caused by another.
    pub transient: bool,
}

impl Error {
    ///
    /// Creates a new instance.
    ///
    pub fn new(code: ErrorCode, msg: String, loc: Loc, transient: bool) -> Self {
        Self {
            code,
            msg,
            solution: None,
            loc,
            transient,
        }
    }

    pub fn new_lex(code: ErrorCode, loc: Loc, asset: Asset<'_>) -> Self {
        let solution = Some(ErrorSolution::new(
            format!("Try removing token '{}'", asset.referenced_slice_for(loc)),
            loc,
        ));

        Self {
            code,
            msg: format!("Unexpected token '{}'", asset.referenced_slice_for(loc)),
            solution,
            loc,
            transient: false,
        }
    }

    ///
    /// Creates a new error for missing a type, provinding a fix if a gty exists.
    ///
    pub fn new_ty_missing(
        code: ErrorCode,
        msg: String,
        loc: Loc,
        asset: Asset<'_>,
        gty_loc: Option<Loc>,
    ) -> Self {
        let solution = gty_loc.map(|gty_loc| {
            ErrorSolution::new(
                format!(
                    "Try including '{}'",
                    asset.source_map().get_asset_for_loc(gty_loc).alias
                ),
                Loc::new(0, 1, 1),
            )
        });

        Self {
            code,
            msg,
            solution,
            loc,
            transient: false,
        }
    }

    ///
    /// Prints the error to stderr (colored) using termcolor.
    ///
    pub fn print(&self, smap: &SourceMap) -> std::io::Result<()> {
        let mut stream = StandardStream::stderr(ColorChoice::Always);
        let asset = smap.get_asset_for_loc(self.loc);

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
            asset.path.to_str().unwrap(),
            self.loc.line
        )?;

        stream.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
        write!(&mut stream, "    | ")?;
        stream.reset()?;

        let mut line_drawn = false;

        for c in smap.padded_referenced_slice_for(self.loc).chars() {
            write!(&mut stream, "{}", c)?;
            if c == '\n' {
                stream.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;

                if line_drawn {
                    write!(&mut stream, "    | ")?;
                } else {
                    write!(&mut stream, "{:>3} | ", self.loc.line)?;
                    line_drawn = true
                }
                stream.reset()?;
            }
        }

        writeln!(&mut stream)?;

        if let Some(solution) = &self.solution {
            let solution_asset = smap.get_asset_for_loc(solution.loc);

            stream.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
            writeln!(&mut stream, "    = {}", solution.msg)?;
            writeln!(
                &mut stream,
                "       in {}:{}",
                solution_asset.path.to_str().unwrap(),
                solution.loc.line
            )?;
        }

        stream.reset()
    }
}

///
/// Classes of NDL errors.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ParUnexpectedEOF = 0,
    TooManyErrors = 1,

    ParUnexpectedKeyword = 10,

    ParLinkMissingIdentifier = 11,
    ParLinkMissingDefBlockOpen = 12,
    ParLinkMissingDefBlockClose = 13,
    ParLinkInvalidKeyToken = 14,
    ParLinkInvalidKey = 15,
    ParLinkInvalidKvSeperator = 16,
    ParLinkInvalidValueToken = 17,
    ParLinkInvalidValueType = 18,
    ParLinkIncompleteDefinition = 19,

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

    ParNetworkMissingIdentifer = 50,
    ParNetworkMissingDefBlockOpen = 51,
    ParNetworkMissingSectionIdentifier = 52,
    ParNetworkInvalidSectionIdentifer = 53,
    ParNetworkInvalidSeperator = 54,

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
    TycModuleConNonMatchingGateSizes = 306,
    TycIncludeInvalidAlias = 330,
    TycGateInvalidNullGate = 340,
    TycGateFieldDuplication = 341,
    TycParInvalidType = 360,
    TycParAllreadyDefined = 361,
    TycModuleAllreadyDefined = 362,
    TycLinkAllreadyDefined = 363,

    TycNetworkAllreadyDefined = 370,
    TycnetworkSubmoduleFieldAlreadyDeclared = 371,
    TycNetworkSubmoduleInvalidTy = 373,
    TycNetworkConInvalidChannelTy = 374,
    TycNetworkConUnknownIdentSymbol = 375,
    TycNetworkConIllegalLocalNodeIdent = 376,
    TycNetworkConNonMatchingGateSizes = 377,
    TycNetworkEmptyNetwork = 378,
}
