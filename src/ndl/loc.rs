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
    pub fn new(pos: usize, len: usize, line: usize) -> Self {
        Self { pos, len, line }
    }

    pub fn fromto(from: Self, to: Self) -> Self {
        assert!(from.pos < to.pos);
        let len = (to.pos + to.len) - from.pos;
        Self {
            pos: from.pos,
            line: from.line,
            len,
        }
    }

    pub fn referenced_slice_in<'a>(&self, str: &'a str) -> &'a str {
        &str[self.pos..(self.pos + self.len)]
    }

    pub fn padded_referenced_slice_in<'a>(&self, str: &'a str) -> &'a str {
        let mut start_lf = 2;
        let mut start = self.pos;
        while start > 0 && start_lf > 0 {
            start -= 1;
            if str.chars().nth(start).unwrap() == '\n' {
                start_lf -= 1;
            }
        }

        let mut end_lf = 2;
        let mut end = self.pos + self.len - 1;
        while end < str.len() && end_lf > 0 {
            end += 1;
            if str.chars().nth(end).unwrap() == '\n' {
                end_lf -= 1;
            }
        }

        &str[(start + 1)..end]
    }
}
