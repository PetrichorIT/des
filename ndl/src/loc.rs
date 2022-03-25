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
        Self { pos, len, line }
    }

    ///
    /// Creates a new [Loc] describing an objects starting at the 'from'
    /// location and ending at the last character of the 'to' location.
    ///
    pub fn fromto(from: Self, to: Self) -> Self {
        assert!(
            from.pos < to.pos,
            "A instance of Loc must span from a smaller value 'from' to a bigger value 'to'."
        );
        let len = (to.pos + to.len) - from.pos;
        Self {
            pos: from.pos,
            line: from.line,
            len,
        }
    }

    ///
    /// Returns a Loc that point directly after the current loc at a zero-width
    /// token.
    ///
    #[must_use]
    pub fn after(self) -> Self {
        Self::new(self.pos + self.len, 0, self.line)
    }
}
