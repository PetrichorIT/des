#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub(crate) pos: usize,
    pub(crate) len: usize,
}

impl Span {
    pub fn new(pos: usize, len: usize) -> Self {
        assert!(len > 0);
        Span { pos, len }
    }

    pub fn fromto(lhs: Span, rhs: Span) -> Self {
        assert!(lhs.pos < rhs.pos, "A span cannot be created in reverse");
        let len = (rhs.pos + rhs.len) - lhs.pos;
        assert!(len > 0);
        Self { pos: lhs.pos, len }
    }

    pub fn after(&self) -> Span {
        Self {
            pos: self.pos + self.len,
            len: 0,
        }
    }
}
