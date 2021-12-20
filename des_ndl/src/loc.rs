use crate::SourceAsset;

use super::SourceAssetDescriptor;

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

    ///
    /// Extracts the raw string slice referenced by the [Loc].
    ///
    pub fn referenced_slice_in<'a>(&self, str: &'a str) -> &'a str {
        &str[self.pos..(self.pos + self.len)]
    }

    ///
    /// Extracts the raw string slice referenced by the [Loc],
    /// padding it with one extra line (aboth and below) from the source.
    ///
    pub fn padded_referenced_slice_in<'a>(&self, asset: &'a SourceAsset) -> &'a str {
        let start_line = self.line;
        let end_line = asset.line_of_pos(self.pos + self.len);

        let padded_start_line = start_line.saturating_sub(1);
        let padded_end_line = (end_line + 1).min(asset.lines);

        let padded_start_pos = asset.line_pos_mapping[padded_start_line];
        let padded_end_pos = asset.line_pos_mapping[padded_end_line];

        &asset.data[padded_start_pos..=padded_end_pos]
    }
}

///
/// A type that contains exact information about its location.
///
pub trait LocAssetEntity {
    fn loc(&self) -> Loc;
    fn asset_descriptor(&self) -> &SourceAssetDescriptor;
}
