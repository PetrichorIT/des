#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorContext {
    pub errors: Vec<Error>,
    transient: bool,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            transient: false,
        }
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn record(&mut self, code: ErrorCode, msg: String, pos: usize, len: usize) {
        self.errors.push(Error {
            code,
            msg,
            pos,
            len,
            transient: self.transient,
        });
        self.transient = true;
    }

    pub fn reset_transient(&mut self) {
        self.transient = false;
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: ErrorCode,
    pub msg: String,
    pub pos: usize,
    pub len: usize,
    pub transient: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    LinkMissingIdentifier = 1,
    LinkMissingDefBlockOpen = 2,
    LinkMissingDefBlockClose = 3,
    LinkInvalidKeyToken = 4,
    LinkInvalidKey = 5,
    LinkInvalidKvSeperator = 6,
    LinkInvalidValueToken = 7,
    LinkInvalidValueType = 8,

    ModuleMissingIdentifer = 20,
    ModuleMissingDefBlockOpen = 21,
    ModuleMissingSectionIdentifier = 23,
    ModuleInvalidSectionIdentifer = 24,
    ModuleInvalidSeperator = 25,
    ModuleInvalidKeyToken = 26,
    ModuleGateMissingClosingBracket = 27,
    ModuleGateInvalidIdentifierToken = 28,
    ModuleGateInvalidGateSize = 29,
    ModuleSubInvalidIdentiferToken = 30,
    ModuleSubInvalidSeperator = 31,
    ModuleConInvalidIdentiferToken = 32,
    ModuleConInvaldiChannelSyntax = 33,

    LiteralIntParseError = 100,
    LiteralFloatParseError = 101,
}
