use std::{io::Write, mem::MaybeUninit};
use termcolor::*;

use crate::loc::LocAssetEntity;

use super::{
    loc::Loc,
    source::{SourceAsset, SourceAssetDescriptor},
};

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
        // TODO
        true
    }
}

impl Default for GlobalErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

///
/// A local error context for creating transient errors
/// during the parsing stage.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalParsingErrorContext<'a> {
    /// A list of the collect errors, including transients.
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
    ///
    /// Creates an error context without asset binding.
    /// This cannot be usied without binding to an asset.
    ///
    pub fn null() -> Self {
        Self {
            errors: Vec::new(),

            asset: unsafe { &*MaybeUninit::<SourceAsset>::uninit().as_ptr() },
            transient: false,
        }
    }

    ///
    /// Creates a new context bound to the given asset.
    ///
    pub fn new(asset: &'a SourceAsset) -> Self {
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
    pub fn record(&mut self, code: ErrorCode, msg: String, loc: Loc) {
        self.errors.push(Error {
            code,
            msg,
            source: loc.padded_referenced_slice_in(&self.asset.data).to_string(),
            solution: None,

            loc,
            asset: self.asset.descriptor.clone(),

            transient: self.transient,
        });
        self.transient = true;
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
    pub asset: SourceAssetDescriptor,
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
    pub fn new(msg: String, loc: Loc, asset: SourceAssetDescriptor) -> Self {
        Self {
            msg: msg,
            loc: loc,
            asset: asset,
        }
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
    /// The souce code lines the error occurred in.
    pub source: String,
    /// A possible solution for the error.
    pub solution: Option<ErrorSolution>,

    /// The descirptor of the asset the error occured in.
    pub asset: SourceAssetDescriptor,
    /// The exact location of the error.
    pub loc: Loc,

    /// A indicator whether this error syntacticly was caused by another.
    pub transient: bool,
}

impl Error {
    ///
    /// Creates a new instance.
    ///
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
            solution: None,

            asset,
            loc,

            transient,
        }
    }

    pub fn new_lex(code: ErrorCode, loc: Loc, asset: &SourceAsset) -> Self {
        let source = loc.padded_referenced_slice_in(&asset.data).to_string();
        let asset_d = asset.descriptor.clone();

        let solution = Some(ErrorSolution::new(
            format!(
                "Try removing token '{}'.",
                loc.referenced_slice_in(&asset.data)
            ),
            loc,
            asset_d.clone(),
        ));

        Self {
            code,
            msg: format!(
                "Unexpected token '{}'",
                loc.referenced_slice_in(&asset.data)
            ),
            source,
            solution,

            asset: asset_d,
            loc,

            transient: false,
        }
    }

    ///
    /// Creates a new error for missing a type, provinding a fix if a gty exists.
    ///
    pub fn new_ty_missing<T: LocAssetEntity>(
        code: ErrorCode,
        msg: String,
        loc: Loc,
        asset: &SourceAsset,
        gty: Option<&&T>,
    ) -> Self {
        let solution = match gty {
            Some(gty) => Some(ErrorSolution::new(
                format!("Try including '{}'.", gty.asset_descriptor().alias),
                Loc::new(0, 0, 1),
                asset.descriptor.clone(),
            )),
            None => None,
        };

        let source = loc.padded_referenced_slice_in(&asset.data).to_string();
        let asset = asset.descriptor.clone();

        Self {
            code,
            msg,
            source,
            solution,

            asset,
            loc,

            transient: false,
        }
    }

    ///
    /// Prints the error to stderr (colored) using termcolor.
    ///
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
        write!(&mut stream, "    | ")?;
        stream.reset()?;

        let mut line_drawn = false;

        for c in self.source.chars() {
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
            stream.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
            writeln!(&mut stream, "    = {}", solution.msg)?;
            writeln!(
                &mut stream,
                "       in {}:{}",
                solution.asset.path.to_str().unwrap(),
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
    TycModuleConNonMatchingGateSizes = 306,
    TycIncludeInvalidAlias = 330,
    TycGateInvalidNullGate = 340,
    TycGateFieldDuplication = 341,
    TycParInvalidType = 360,
    TycParAllreadyDefined = 361,
}
