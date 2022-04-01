use crate::*;

use std::io::Write;
use termcolor::*;

pub use ErrorCode::*;

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
    /// The semantic errors found while desugaring.
    pub desugaring_errors: Vec<Error>,
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
            desugaring_errors: Vec::new(),
            tychecking_errors: Vec::new(),
        }
    }

    ///
    /// Returns a reference to all errors.
    ///
    pub fn all(&self) -> impl Iterator<Item = &Error> {
        self.lexing_errors
            .iter()
            .chain(self.parsing_errors.iter())
            .chain(self.desugaring_errors.iter())
            .chain(self.tychecking_errors.iter())
    }

    ///
    /// Indicates whether an error has occured.
    ///
    pub fn has_errors(&self) -> bool {
        !(self.lexing_errors.is_empty()
            && self.parsing_errors.is_empty()
            && self.desugaring_errors.is_empty()
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
    pub fn record(&mut self, token: &Token) -> NdlResult<()> {
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
    /// Records an error, determining transients automaticly.
    ///
    pub fn record(&mut self, code: ErrorCode, msg: String, loc: Loc) -> NdlResult<()> {
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

    pub fn record_missing_token(
        &mut self,
        code: ErrorCode,
        msg: String,
        token: &Token,
        expected_token: &str,
    ) -> NdlResult<()> {
        let solution = ErrorSolution {
            msg: format!("Try adding '{}'", expected_token),
            loc: token.loc.after(),
        };
        self.errors.push(Error {
            code,
            msg,
            solution: Some(solution),

            loc: token.loc,

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
    ) -> NdlResult<()> {
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

    pub fn new_with_solution(
        code: ErrorCode,
        msg: String,
        loc: Loc,
        transient: bool,
        solution: ErrorSolution,
    ) -> Self {
        Self {
            code,
            msg,
            solution: Some(solution),
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
        source_map: &SourceMap,
        gty_loc: Option<Loc>,
    ) -> Self {
        let solution = gty_loc.map(|gty_loc| {
            ErrorSolution::new(
                format!(
                    "Try including '{}'",
                    source_map.get_asset_for_loc(gty_loc).alias
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
    ParUnexpectedEOF,
    TooManyErrors,

    ParUnexpectedKeyword,

    ParLinkMissingIdentifier,
    ParLinkMissingDefBlockOpen,
    ParLinkMissingDefBlockClose,
    ParLinkInvalidKeyToken,
    ParLinkInvalidKey,
    ParLinkInvalidKvSeperator,
    ParLinkInvalidValueToken,
    ParLinkInvalidValueType,
    ParLinkIncompleteDefinition,

    ParModuleMissingIdentifer,
    ParModuleMissingDefBlockOpen,
    ParModuleMissingSectionIdentifier,
    ParModuleInvalidSectionIdentifer,
    ParModuleInvalidSeperator,
    ParModuleInvalidKeyToken,

    ParModuleGateMissingClosingBracket,
    ParModuleGateInvalidIdentifierToken,
    ParModuleGateInvalidGateSize,
    ParModuleGateInvalidServiceAnnotation,

    ParModuleSubInvalidIdentiferToken,
    ParModuleSubInvalidSeperator,
    ParModuleSubInvalidClusterLiteral,
    ParModuleSubInvalidClusterDotChain,
    ParModuleSubMissingClosingBracket,

    ParModuleConInvalidIdentiferToken,
    ParModuleConInvaldiChannelSyntax,
    ParModuleConMissingClosingBracketForCLusterIdent,

    ParAliasMissingIdent,
    ParAliasMissingLikeToken,
    ParAliasMissingLikeKeyword,
    ParAliasMissingPrototypeIdent,

    ParProtoImplInvalidIdent,
    ParProtoImplExpectedEq,
    ParProtoImplAtSomeDef,

    ParNetworkMissingIdentifer,
    ParNetworkMissingDefBlockOpen,
    ParNetworkMissingSectionIdentifier,
    ParNetworkInvalidSectionIdentifer,
    ParNetworkInvalidSeperator,
    ParNetworkDoesntAllowSome,

    ParExpectedIntLiteral,
    ParLiteralIntParseError,
    ParExpectedFloatLiteral,
    ParLiteralFloatParseError,

    LexInvalidSouceToken,
    LexInvalidSouceIdentifier,

    DsgIncludeInvalidAlias,
    DsgDefNameCollision,
    DsgConGateSizedToNotMatch,
    DsgConInvalidChannel,
    DsgConInvalidLocalGateIdent,
    DsgConInvalidGateSize,
    DsgConInvalidField,

    DsgInvalidPrototype,
    DsgProtoImplForNonProtoValue,
    DsgProtoImplMissingField,
    DsgProtoImplTyMissing,
    DsgProtoImplAssociatedTyNotDerivedFromProto,
    DsgProtoImlMissing,

    TycGateConnectionViolatesAnnotation,

    TycDefNameCollission,
    TycModuleSubmoduleFieldAlreadyDeclared,
    TycModuleSubmoduleRecrusiveTyDefinition,
    TycModuleSubmoduleInvalidTy,
    TycModuleConInvalidChannelTy,
    TycModuleConUnknownIdentSymbol,
    TycModuleConNonMatchingGateSizes,
    TycIncludeInvalidAlias,
    TycGateInvalidNullGate,
    TycGateFieldDuplication,
    TycParInvalidType,
    TycParAllreadyDefined,
    TycModuleAllreadyDefined,
    TycLinkAllreadyDefined,

    TycNetworkAllreadyDefined,
    TycnetworkSubmoduleFieldAlreadyDeclared,
    TycNetworkSubmoduleInvalidTy,
    TycNetworkConInvalidChannelTy,
    TycNetworkConUnknownIdentSymbol,
    TycNetworkConIllegalLocalNodeIdent,
    TycNetworkConNonMatchingGateSizes,
    TycNetworkEmptyNetwork,
}
