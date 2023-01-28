mod stream;
mod symbol;

pub struct Token {}

pub enum Delimiter {
    Parenthesis,
    Brace,
    Bracket,
    Invisible,
}

pub enum LitKind {
    Integer,
    Float,
    Str,
}
