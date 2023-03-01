#![allow(unused)]

use std::{
    fs::File,
    io::{Error, ErrorKind, Read, Result},
    path::{Path, PathBuf},
};

pub(crate) mod fs;

mod span;
pub use self::span::*;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct SourceMap {
    pub(crate) buffer: String,
    pub(crate) assets: Vec<SourceMappedAsset>,
}

#[derive(Debug)]
pub(crate) struct SourceMappedAsset {
    pub offset: usize,
    pub len: usize,
    pub ident: AssetIdentifier,
    pub line_pos_mapping: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetIdentifier {
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

impl AssetIdentifier {
    fn root_path(&self) -> Option<&Path> {
        match self {
            Self::Raw { .. } => None,
            Self::Root { path, .. } => Some(path),
            Self::Included { path, .. } => Some(path),
        }
    }
}

impl From<&str> for AssetIdentifier {
    fn from(value: &str) -> Self {
        if value.starts_with("raw:") {
            return AssetIdentifier::Raw {
                alias: value[5..].to_string(),
            };
        }

        let path = PathBuf::from(&value[9..]);
        if value.starts_with("include:") {
            return AssetIdentifier::Included {
                alias: path.to_str().unwrap().to_string(),
                path,
                include: Span::new(0, 0),
            };
        }

        AssetIdentifier::Root {
            alias: path.to_str().unwrap().to_string(),
            path,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Asset<'a> {
    map: &'a SourceMap,
    mapping: &'a SourceMappedAsset,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            assets: Vec::new(),
        }
    }

    pub fn load_file(&mut self, ident: impl Into<AssetIdentifier>) -> Result<Asset<'_>> {
        let ident = ident.into();
        let mut file = File::open(ident.path()?)?;
        let offset = self.buffer.len();
        let n = file.read_to_string(&mut self.buffer)?;

        let mapping = SourceMappedAsset::new(ident, offset, n, self);
        self.assets.push(mapping);

        Ok(Asset {
            map: self,
            mapping: self.assets.last().unwrap(),
        })
    }

    pub fn load_raw(&mut self, ident: impl Into<AssetIdentifier>, raw: &str) -> Asset<'_> {
        let ident = ident.into();
        assert!(matches!(ident, AssetIdentifier::Raw { .. }));

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

    pub(crate) fn span_for(&self, asset: &AssetIdentifier) -> Option<Span> {
        self.assets
            .iter()
            .find(|a| a.ident == *asset)
            .map(|v| v.span())
    }

    pub(crate) fn slice_for(&self, span: Span) -> &str {
        &self.buffer[span.pos..(span.pos + span.len)]
    }

    pub(crate) fn slice_padded_for(&self, span: Span) -> (&str, usize) {
        // println!("{span:?}");

        let asset = self
            .asset_for(span)
            .expect("Failed to assign asset to span");
        let bounds = (asset.offset, asset.offset + asset.len);
        // println!("{}: {:?}", asset.offset, asset.line_pos_mapping);

        let line_start = asset.line_for(span.pos);
        let line_end = asset.line_for(span.pos + span.len);
        let len = line_end - line_start + 1;
        let pad = if len > 2 { 1 } else { 2 };

        let lso = line_start;

        let line_start = line_start.saturating_sub(pad);
        let line_end = line_end
            .saturating_add(pad)
            .min(asset.line_pos_mapping.len() - 1);

        let line_offset = if line_start != 0 {
            lso - line_start
        } else {
            lso - line_start - 1
        };

        let start = *asset
            .line_pos_mapping
            .get(line_start.saturating_sub(1))
            .unwrap_or(&bounds.0)
            .max(&bounds.0);
        let end = (asset.line_pos_mapping[line_end] - 1).min(bounds.1);

        (&self.buffer[start..end], line_offset)
    }

    pub(crate) fn asset(&self, ident: &AssetIdentifier) -> Option<&SourceMappedAsset> {
        self.assets.iter().find(|a| a.ident == *ident)
    }

    pub(crate) fn asset_for(&self, span: Span) -> Option<&SourceMappedAsset> {
        self.assets.iter().find(|asset| asset.contains(span))
    }

    pub(crate) fn line_for(&self, span: Span) -> Option<usize> {
        let asset = self.asset_for(span)?;
        Some(asset.line_for(span.pos))
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
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

    pub fn include_for(&self, other: &SourceMappedAsset) -> Option<String> {
        let s = self.ident.alias();
        let o = other.ident.alias();

        let s = s.split('/').collect::<Vec<_>>();
        let o = o.split('/').collect::<Vec<_>>();

        let n = s.len().min(o.len());
        for i in 0..n {
            if s[i] != o[i] {
                let mut include = String::new();
                let up = n - i;
                for _ in 0..up {
                    include.push_str("../");
                }
                for &e in o.iter().skip(i) {
                    include.push_str(e);
                    include.push('/');
                }
                include.pop();
                return Some(include);
            }
        }

        None
    }

    pub(crate) fn span(&self) -> Span {
        Span::new(self.offset, self.len)
    }

    pub(crate) fn contains(&self, span: Span) -> bool {
        let end = span.pos + span.len;
        self.offset <= span.pos && end <= self.offset + self.len
    }

    pub(crate) fn line_for(&self, pos: usize) -> usize {
        for i in 0..self.line_pos_mapping.len() {
            if self.line_pos_mapping[i] > pos {
                return i;
            }
        }

        self.line_pos_mapping.len()
    }
}

impl AssetIdentifier {
    pub fn raw(s: &str) -> Self {
        Self::Raw {
            alias: s.to_string(),
        }
    }

    pub fn alias(&self) -> &str {
        match self {
            Self::Raw { alias } | Self::Root { alias, .. } | Self::Included { alias, .. } => alias,
        }
    }

    pub(crate) fn relative_asset_alias(&self, path: &Path) -> String {
        let common = fs::common_path(self.path().unwrap(), path);
        let sub = fs::strip_prefix(path, common);
        let sub = sub.with_extension("").to_string_lossy().to_string();
        sub
    }

    pub(crate) fn path(&self) -> Result<&PathBuf> {
        match self {
            Self::Raw { .. } => Err(Error::new(ErrorKind::Other, "asset is not io-bound")),
            Self::Included { path, .. } => Ok(path),
            Self::Root { path, .. } => Ok(path),
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

    pub(crate) fn slice_padded_for(&self, span: Span) -> (&str, usize) {
        self.map.slice_padded_for(span)
    }
}
