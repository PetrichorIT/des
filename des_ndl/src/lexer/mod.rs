use crate::*;

use crate::error::{GlobalErrorContext, LexingErrorContext};

use self::LiteralKind::*;
use self::TokenKind::*;
use cursor::Cursor;

mod cursor;
mod tests;
mod token_stream;

pub use self::token_stream::TokenStream;

/// Creates an iterator that produces tokens from the input string.
pub fn tokenize(input: &str, start_idx: usize) -> impl Iterator<Item = Token> + '_ {
    let mut cursor = Cursor::new(input, start_idx);
    std::iter::from_fn(move || {
        if cursor.is_eof() {
            None
        } else {
            cursor.reset_len_consumed();
            Some(cursor.advance_token())
        }
    })
}

///
/// Performs the entire lexing process for a given asset.
///
pub fn tokenize_and_validate(
    asset: Asset<'_>,
    global_ectx: &mut GlobalErrorContext,
) -> NdlResult<TokenStream> {
    let mut ectx = LexingErrorContext::new(asset);

    let token_stream = TokenStream::new(asset, &mut ectx)?;
    global_ectx.lexing_errors.append(&mut ectx.finish());

    Ok(token_stream)
}

///
/// A raw syntactical element in of the given source asset.
/// Note that the [Loc] always references the [SourceMap] buffer
/// not the relative position in the current asset.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The type of token encountered.
    pub kind: TokenKind,
    /// The location of the token in the source asset.
    pub loc: Loc,
}

impl Token {
    ///
    /// Creates a new token from the given raw parameters.
    ///
    pub fn new(kind: TokenKind, pos: usize, len: usize, line: usize) -> Self {
        Self {
            kind,
            loc: Loc::new(pos, len, line),
        }
    }
}

///
/// The tokens type.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenKind {
    // A single line comment
    Comment,
    /// Any whitespace characters sequence.
    Whitespace,
    /// Any token that can be either type, var name or keyqord
    Ident,
    /// A invalid identifer.
    InvalidIdent,
    /// A nonterminated token prefix.
    UnknownPrefix,
    /// A literal value.
    Literal {
        kind: LiteralKind,
        suffix_start: usize,
    },

    /// ";"
    Semi,
    /// ","
    Comma,
    /// "."
    Dot,
    /// "("
    OpenParen,
    /// ")"
    CloseParen,
    /// "{"
    OpenBrace,
    /// "}"
    CloseBrace,
    /// "["
    OpenBracket,
    /// "]"
    CloseBracket,
    /// "@"
    At,
    /// "#"
    Pound,
    /// "~"
    Tilde,
    /// "?"
    Question,
    /// ":"
    Colon,
    /// "$"
    Dollar,
    /// "="
    Eq,
    /// "!"
    Bang,
    /// "<"
    Lt,
    /// ">"
    Gt,
    /// "-"
    Minus,
    /// "&"
    And,
    /// "|"
    Or,
    /// "+"
    Plus,
    /// "*"
    Star,
    /// "/"
    Slash,
    /// "^"
    Caret,
    /// "%"
    Percent,

    /// Unknown token, not expected by the lexer, e.g. "№"
    Unknown,
}

impl TokenKind {
    ///
    /// Indicates whether this token is nessecary for the parser,
    /// assuming the token is valid.
    ///
    pub fn reducable(&self) -> bool {
        matches!(self, Comment)
    }

    ///
    /// Indicates whether the token is valid in a NDL file.
    /// If not it will be stripped from the token stream,
    /// and ignored in further processing (generally.)
    ///
    pub fn valid(&self) -> bool {
        matches!(
            self,
            Comment
                | Whitespace
                | Ident
                | Literal { .. }
                | OpenBrace
                | OpenBracket
                | CloseBrace
                | CloseBracket
                | Colon
                | Lt
                | Gt
                | Minus
                | Slash
                | Dot
        )
    }
}

///
/// A literal value definition token.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralKind {
    Int { base: Base, empty_int: bool },
    Float { base: Base, empty_exp: bool },
}

///
/// The numeric bases numbers can be wirtten in.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Base {
    Binary,
    Octal,
    Hexadecimal,
    Decimal,
}

impl Base {
    pub fn radix(&self) -> u32 {
        match self {
            Base::Binary => 2,
            Base::Octal => 8,
            Base::Decimal => 10,
            &Base::Hexadecimal => 16,
        }
    }
}

/// True if `c` is considered a whitespace.
pub fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        // Usual ASCII suspects
        '\u{0009}'   // \t
        | '\u{000A}' // \n
        | '\u{000B}' // vertical tab
        | '\u{000C}' // form feed
        | '\u{000D}' // \r
        | '\u{0020}' // space

        // NEXT LINE from latin1
        | '\u{0085}'

        // Bidi markers
        | '\u{200E}' // LEFT-TO-RIGHT MARK
        | '\u{200F}' // RIGHT-TO-LEFT MARK

        // Dedicated whitespace characters from Unicode
        | '\u{2028}' // LINE SEPARATOR
        | '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

/// True if `c` is valid as a first character of an identifier.
pub fn is_id_start(c: char) -> bool {
    // This is XID_Start OR '_' (which formally is not a XID_Start).
    c == '_' || unicode_xid::UnicodeXID::is_xid_start(c)
}

/// True if `c` is valid as a non-first character of an identifier.
pub fn is_id_continue(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_continue(c)
}

// /// The passed string is lexically an identifier.
// pub fn is_ident(string: &str) -> bool {
//     let mut chars = string.chars();
//     if let Some(start) = chars.next() {
//         is_id_start(start) && chars.all(is_id_continue)
//     } else {
//         false
//     }
// }

impl Cursor<'_> {
    /// Parses a token from the input string.
    fn advance_token(&mut self) -> Token {
        let line = self.line;

        let first_char = self.bump().unwrap();
        let token_kind = match first_char {
            // Slash, comment or block comment.
            '/' => match self.first() {
                '/' => self.line_comment(),
                _ => Slash,
            },

            c if is_id_start(c) => self.ident_or_unknown_prefix(),

            // Whitespace sequence.
            c if is_whitespace(c) && c == '\n' => {
                self.line += 1;
                self.whitespace()
            }
            c if is_whitespace(c) => self.whitespace(),

            // Numeric literal.
            c @ '0'..='9' => {
                let literal_kind = self.number(c);
                let suffix_start = self.len_consumed();
                self.eat_literal_suffix();
                TokenKind::Literal {
                    kind: literal_kind,
                    suffix_start,
                }
            }

            // One-symbol tokens.
            ';' => Semi,
            ',' => Comma,
            '.' => Dot,
            '(' => OpenParen,
            ')' => CloseParen,
            '{' => OpenBrace,
            '}' => CloseBrace,
            '[' => OpenBracket,
            ']' => CloseBracket,
            '@' => At,
            '#' => Pound,
            '~' => Tilde,
            '?' => Question,
            ':' => Colon,
            '$' => Dollar,
            '=' => Eq,
            '!' => Bang,
            '<' => Lt,
            '>' => Gt,
            '-' => Minus,
            '&' => And,
            '|' => Or,
            '+' => Plus,
            '*' => Star,
            '^' => Caret,
            '%' => Percent,

            _ => Unknown,
        };
        let idx = self.idx;
        self.idx += self.len_consumed();

        Token::new(token_kind, idx, self.len_consumed(), line)
    }

    fn line_comment(&mut self) -> TokenKind {
        debug_assert!(self.prev() == '/' && self.first() == '/');
        self.bump();

        self.eat_while(|c| c != '\n');
        Comment
    }

    fn whitespace(&mut self) -> TokenKind {
        debug_assert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        Whitespace
    }

    fn ident_or_unknown_prefix(&mut self) -> TokenKind {
        debug_assert!(is_id_start(self.prev()));
        // Start is already eaten, eat the rest of identifier.
        self.eat_while(is_id_continue);
        // Known prefixes must have been handled earlier. So if
        // we see a prefix here, it is definitely an unknown prefix.
        match self.first() {
            '#' | '"' | '\'' => UnknownPrefix,
            c if !c.is_ascii() && unic_emoji_char::is_emoji(c) => {
                self.fake_ident_or_unknown_prefix()
            }
            _ => Ident,
        }
    }

    fn fake_ident_or_unknown_prefix(&mut self) -> TokenKind {
        // Start is already eaten, eat the rest of identifier.
        self.eat_while(|c| {
            unicode_xid::UnicodeXID::is_xid_continue(c)
                || (!c.is_ascii() && unic_emoji_char::is_emoji(c))
                || c == '\u{200d}'
        });
        // Known prefixes must have been handled earlier. So if
        // we see a prefix here, it is definitely an unknown prefix.
        match self.first() {
            '#' | '"' | '\'' => UnknownPrefix,
            _ => InvalidIdent,
        }
    }

    fn number(&mut self, first_digit: char) -> LiteralKind {
        debug_assert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = Base::Decimal;
        if first_digit == '0' {
            // Attempt to parse encoding base.
            let has_digits = match self.first() {
                'b' => {
                    base = Base::Binary;
                    self.bump();
                    self.eat_decimal_digits()
                }
                'o' => {
                    base = Base::Octal;
                    self.bump();
                    self.eat_decimal_digits()
                }
                'x' => {
                    base = Base::Hexadecimal;
                    self.bump();
                    self.eat_hexadecimal_digits()
                }
                // Not a base prefix.
                '0'..='9' | '_' | '.' | 'e' | 'E' => {
                    self.eat_decimal_digits();
                    true
                }
                // Just a 0.
                _ => {
                    return Int {
                        base,
                        empty_int: false,
                    }
                }
            };
            // Base prefix was provided, but there were no digits
            // after it, e.g. "0x".
            if !has_digits {
                return Int {
                    base,
                    empty_int: true,
                };
            }
        } else {
            // No base prefix, parse number in the usual way.
            self.eat_decimal_digits();
        };

        match self.first() {
            // Don't be greedy if this is actually an
            // integer literal followed by field/method access or a range pattern
            // (`0..2` and `12.foo()`)
            '.' if self.second() != '.' && !is_id_start(self.second()) => {
                // might have stuff after the ., and if it does, it needs to start
                // with a number
                self.bump();
                let mut empty_exp = false;
                if self.first().is_digit(10) {
                    self.eat_decimal_digits();
                    match self.first() {
                        'e' | 'E' => {
                            self.bump();
                            empty_exp = !self.eat_float_exponent();
                        }
                        _ => (),
                    }
                }
                Float { base, empty_exp }
            }
            'e' | 'E' => {
                self.bump();
                let empty_exp = !self.eat_float_exponent();
                Float { base, empty_exp }
            }
            _ => Int {
                base,
                empty_int: false,
            },
        }
    }

    fn eat_decimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    fn eat_hexadecimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    fn eat_float_exponent(&mut self) -> bool {
        debug_assert!(self.prev() == 'e' || self.prev() == 'E');
        if self.first() == '-' || self.first() == '+' {
            self.bump();
        }
        self.eat_decimal_digits()
    }

    fn eat_literal_suffix(&mut self) {
        self.eat_identifier();
    }
    fn eat_identifier(&mut self) {
        if !is_id_start(self.first()) {
            return;
        }
        self.bump();

        self.eat_while(is_id_continue);
    }
}
