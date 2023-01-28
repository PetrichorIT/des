use std::{
    fs::File,
    io::{Error, ErrorKind, Read, Result},
    path::{Path, PathBuf},
};

mod span;
pub use self::span::*;

#[cfg(test)]
mod tests;

pub(crate) struct SourceMap {
    buffer: String,
    assets: Vec<SourceMappedAsset>,
}

pub(crate) struct SourceMappedAsset {
    pub offset: usize,
    pub len: usize,
    pub ident: AssetIdentifier,
    pub line_pos_mapping: Vec<usize>,
}

pub(crate) enum AssetIdentifier {
    Raw {
        alias: String,
    },
    Root {
        path: PathBuf,
        alias: String,
    },
    Included {
        path: PathBuf,
        alias: String,
        include: Span,
    },
}

pub struct Asset<'a> {
    map: &'a SourceMap,
    mapping: &'a SourceMappedAsset,
}

impl SourceMap {
    pub(crate) fn new() -> Self {
        Self {
            buffer: String::new(),
            assets: Vec::new(),
        }
    }

    pub(crate) fn load_file(&mut self, ident: AssetIdentifier) -> Result<Asset<'_>> {
        let mut file = File::open(&ident.path()?)?;
        let offset = self.buffer.len();
        let n = file.read_to_string(&mut self.buffer)?;

        let mapping = SourceMappedAsset::new(ident, offset, n, self);
        self.assets.push(mapping);

        Ok(Asset {
            map: self,
            mapping: self.assets.last().unwrap(),
        })
    }

    pub(crate) fn load_raw(&mut self, ident: AssetIdentifier, raw: &str) -> Asset<'_> {
        let offset = self.buffer.len();
        let n = raw.len();
        self.buffer.push_str(raw);

        let mapping = SourceMappedAsset::new(ident, offset, n, self);
        self.assets.push(mapping);

        Asset {
            map: self,
            mapping: self.assets.last().unwrap(),
        }
    }

    pub(crate) fn slice_for(&self, span: Span) -> &str {
        &self.buffer[span.pos..(span.pos + span.len)]
    }

    pub(crate) fn slice_padded_for(&self, span: Span) -> &str {
        let asset = self
            .asset_for(span)
            .expect("Failed to assign asset to span");
        let bounds = (asset.offset, asset.offset + asset.len);

        let line_start = asset.line_for(span.pos);
        let line_end = asset.line_for(span.pos + span.len);

        let line_start = line_start.saturating_sub(1);
        let line_end = line_end.saturating_add(1).min(asset.line_pos_mapping.len());

        let start = asset.line_pos_mapping[line_start].max(bounds.0);
        let end = (asset.line_pos_mapping[line_end] + 1).min(bounds.1);

        &self.buffer[start..end]
    }

    pub(crate) fn asset_for(&self, span: Span) -> Option<&SourceMappedAsset> {
        for asset in &self.assets {
            if asset.contains(span) {
                return Some(asset);
            }
        }

        None
    }
}

impl SourceMappedAsset {
    pub(crate) fn new(ident: AssetIdentifier, offset: usize, len: usize, map: &SourceMap) -> Self {
        let data = &map.buffer[offset..(offset + len)];

        // pos is a mapping line-start-index --> line-number (index)

        let mut idx = 0;
        let mut pos = vec![0];

        for c in data.chars() {
            if c == '\n' {
                pos.push(offset + idx + 1);
            }
            idx += c.len_utf8();
        }

        Self {
            ident,
            offset,
            len,
            line_pos_mapping: pos,
        }
    }

    pub(crate) fn contains(&self, span: Span) -> bool {
        let end = span.pos + span.len;
        self.offset <= span.pos && end <= self.offset + self.len
    }

    pub(crate) fn line_for(&self, pos: usize) -> usize {
        match self.line_pos_mapping.binary_search(&pos) {
            Ok(n) => n,
            Err(n) => {
                if n >= self.line_pos_mapping.len() {
                    n - 1
                } else {
                    n
                }
            }
        }
    }
}

impl AssetIdentifier {
    pub(crate) fn raw(s: &str) -> Self {
        Self::Raw {
            alias: s.to_string(),
        }
    }

    pub(crate) fn alias(&self) -> &str {
        match self {
            Self::Raw { alias } | Self::Root { alias, .. } | Self::Included { alias, .. } => alias,
        }
    }

    pub(crate) fn path(&self) -> Result<&PathBuf> {
        match self {
            Self::Raw { .. } => Err(Error::new(ErrorKind::Other, "asset is not io-bound")),
            Self::Included { path, .. } => Ok(&path),
            Self::Root { path, .. } => Ok(&path),
        }
    }
}

impl<'a> Asset<'a> {
    pub(crate) fn new(map: &'a SourceMap, mapping: &'a SourceMappedAsset) -> Self {
        Self { map, mapping }
    }

    pub(crate) fn alias(&self) -> &str {
        self.mapping.ident.alias()
    }

    pub(crate) fn source_span(&self) -> Span {
        Span::new(self.mapping.offset, self.mapping.len)
    }

    pub(crate) fn source(&self) -> &'a str {
        &self.map.buffer[self.mapping.offset..(self.mapping.offset + self.mapping.len)]
    }

    pub(crate) fn slice_for(&self, span: Span) -> &str {
        self.map.slice_for(span)
    }

    pub(crate) fn slice_padded_for(&self, span: Span) -> &str {
        self.map.slice_padded_for(span)
    }
}
