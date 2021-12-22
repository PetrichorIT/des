#[derive(Debug, Clone, Copy, PartialEq, Eq)]
///
/// The syntactic placement of a token or definition in
/// the souce.
///
pub struct Loc {
    /// The index of the first char of the object.
    pub pos: usize,
    /// The length of the objects souce (including reducables).
    pub len: usize,
    /// The line number of the given token
    pub line: usize,
}

impl Loc {
    ///
    /// Creates a new unchecked instance using the given values.
    ///
    pub fn new(pos: usize, len: usize, line: usize) -> Self {
        assert_ne!(
            len, 0,
            "Any described object must have at least one associated source character"
        );
        Self { pos, len, line }
    }

    ///
    /// Creates a new [Loc] describing an objects starting at the 'from'
    /// location and ending at the last character of the 'to' location.
    ///
    pub fn fromto(from: Self, to: Self) -> Self {
        assert!(from.pos < to.pos);
        let len = (to.pos + to.len) - from.pos;
        Self {
            pos: from.pos,
            line: from.line,
            len,
        }
    }
}
