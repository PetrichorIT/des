use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

use crate::Loc;

///
/// A central buffer to store and manage all loaded assets.
/// The central reference point for [Loc] objects
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    buffer: String,
    buffer_descriptors: Vec<MappedAssetDescriptor>,
}

impl SourceMap {
    /// The byte size of the buffer.
    pub fn len_bytes(&self) -> usize {
        self.buffer.len()
    }

    /// The number of assets loaded into the buffer.
    pub fn len_assets(&self) -> usize {
        self.buffer_descriptors.len()
    }

    /// The assets loaded into the buffer.
    pub fn mapped_assets(&self) -> &Vec<MappedAssetDescriptor> {
        &self.buffer_descriptors
    }

    ///
    /// A function that returns a loaded asset descriptor if
    /// any was found based on the alias.
    ///
    pub fn get_asset(&self, alias: &str) -> Option<Asset<'_>> {
        let asset = self.buffer_descriptors.iter().find(|a| a.alias == alias)?;
        Some(Asset::new(self, asset))
    }

    ///
    /// A function that retursn a loaded asset descriptor based
    /// on a [Loc] that the descriptor should be resposible for.
    ///
    pub fn get_asset_for_loc(&self, loc: Loc) -> &MappedAssetDescriptor {
        self.buffer_descriptors
            .iter()
            .find(|d| d.pos <= loc.pos && (d.pos + d.len) >= (loc.pos + loc.len))
            .expect("Failed to find referenced Loc in Source Map")
    }

    ///
    /// Creates a new empty [SourceMap].
    ///
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            buffer_descriptors: Vec::new(),
        }
    }

    ///
    /// Tries to load a new asset into the [SourceMap] using an
    /// [AssetDescriptor] to find the asset.
    /// Returns an Asset mapping to the loaded asset.
    ///
    pub fn load(&mut self, descriptor: AssetDescriptor) -> std::io::Result<Asset<'_>> {
        // Note:
        // Usage of indices instead of direct ref is nessecary
        // because of E0502 (rustc isnt smart enough yet)
        if let Some(dsc_idx) = self
            .buffer_descriptors
            .iter()
            .enumerate()
            .find(|(_, d)| d.alias == descriptor.alias && d.path == descriptor.path)
            .map(|(i, _)| i)
        {
            Ok(Asset::new(self, &self.buffer_descriptors[dsc_idx]))
        } else {
            let mut file = File::open(&descriptor.path)?;
            let byte_len = file.metadata()?.len() as usize;

            let pos = self.buffer.len();
            file.read_to_string(&mut self.buffer)?;

            let descriptor =
                MappedAssetDescriptor::new(descriptor, pos, &self.buffer[pos..(pos + byte_len)]);

            self.buffer_descriptors.push(descriptor);
            Ok(Asset::new(self, self.buffer_descriptors.last().unwrap()))
        }
    }

    // === Accessors ===

    ///
    /// Returns the slice referenced by the [Loc].
    ///
    pub fn referenced_slice_for(&self, loc: Loc) -> &str {
        &self.buffer[loc.pos..(loc.pos + loc.len)]
    }

    ///
    /// Returns the slice referenced by the [Loc] with two additional
    /// lines of padding, one above and one below the original slice.
    ///
    pub fn padded_referenced_slice_for(&self, loc: Loc) -> &str {
        let asset = self.get_asset_for_loc(loc);
        self.padded_referenced_slice_for_with_asset(loc, asset)
    }

    ///
    /// Returns the slice referenced by the [Loc] with two additional
    /// lines of padding, one above and one below the original slice.
    /// This functions uses the [MappedAssetDescriptor] to determine the line pos.
    ///
    fn padded_referenced_slice_for_with_asset(
        &self,
        loc: Loc,
        asset: &MappedAssetDescriptor,
    ) -> &str {
        // println!("{:?} {}", loc, self.referenced_slice_for(loc));

        let start_line = loc.line;
        let end_line = asset.line_of_pos(loc.pos + loc.len);

        // println!("{} \t{}", start_line, end_line);

        let padded_start_line = start_line.saturating_sub(1);
        let padded_end_line = (end_line + 1).min(asset.len_lines);

        // println!("{} \t{}", padded_start_line, padded_end_line);

        let padded_start_pos = asset.line_pos_mapping[padded_start_line] + asset.pos;
        let padded_end_pos = asset.line_pos_mapping[padded_end_line] + 1 + asset.pos;

        // println!("{} \t{}", padded_start_pos, padded_end_pos);

        &self.buffer[padded_start_pos..padded_end_pos]
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

///
/// A managment block for an asset loaded into a [SourceMap].
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedAssetDescriptor {
    /// The byte position of the given asset.
    pub pos: usize,
    /// The byte length of the given asset.
    pub len: usize,

    /// The source path for the asset.
    pub path: PathBuf,
    /// The internal alias.
    pub alias: String,

    /// The number of characters in the asset.
    pub len_chars: usize,
    /// The number of lines in the asset.
    pub len_lines: usize,
    /// A mapping lines (index) --> pos (first char of each line).
    pub line_pos_mapping: Vec<usize>,
}

impl MappedAssetDescriptor {
    ///
    /// Creates a new [MappedAssetDescriptor] using a normal [AssetDescriptor]
    /// the data slice and a pos.
    ///
    pub fn new(descriptor: AssetDescriptor, pos: usize, data: &str) -> Self {
        let mut idx = 0;
        let mut len_lines = 1;
        let mut len_chars = 0;
        let mut line_pos_mapping = vec![0; 2];

        for c in data.chars() {
            if c == '\n' {
                len_lines += 1;
                line_pos_mapping.push(idx + 1);
            }

            idx += c.len_utf8();
            len_chars += 1;
        }

        let AssetDescriptor { path, alias } = descriptor;

        Self {
            pos,
            len: data.len(),
            path,
            alias,

            len_chars,
            len_lines,
            line_pos_mapping,
        }
    }

    ///
    /// Maps a absolute position to an internal line in the asset.
    ///
    pub fn line_of_pos(&self, pos: usize) -> usize {
        assert!(self.pos <= pos && pos <= self.pos + self.len);
        let rel_pos = pos.saturating_sub(self.pos);
        for (line, line_start) in self.line_pos_mapping.iter().enumerate() {
            if *line_start > rel_pos {
                // return line;
                return line.saturating_sub(1).max(1);
            }
        }

        self.len_lines
    }
}

///
/// A descriptor of an asset based on it filepath (relative to WORK_DIR)
/// and its internal alias.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetDescriptor {
    /// The path to the given asset.
    pub path: PathBuf,
    /// The simplified call path used in includes.
    pub alias: String,
}

impl AssetDescriptor {
    ///
    /// Creates a new unchecked asset descriptor.
    ///
    pub fn new(path: PathBuf, alias: String) -> Self {
        Self { path, alias }
    }

    ///
    /// Creates a new asset descriptor by deriving the alias from the
    /// path and the relative workspace root.
    ///
    pub fn from_path(path: PathBuf, rel_root: &Path) -> Self {
        // Single file mode
        if path == rel_root {
            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
            return Self::new(path, file_name);
        }

        let components = path.components().collect::<Vec<Component>>();
        let naming_subset = &components[rel_root.components().count()..]
            .iter()
            .filter_map(|c| match c {
                Component::Normal(str) => Some(str.to_str()?),
                _ => None,
            })
            .collect::<Vec<&str>>();

        let mut alias = naming_subset.join("/");
        alias.truncate(alias.len() - 4);

        Self::new(path, alias)
    }

    ///
    /// Create a new asset descriptor by deriving the path from the
    /// alias and the relative workspace root.
    pub fn from_alias(alias: String, rel_root: &Path) -> Self {
        let mut path = rel_root.to_path_buf();

        for split in alias.split('/') {
            path.push(split)
        }
        path.set_extension(".ndl");

        Self { path, alias }
    }
}

///
/// A temporary asset mapping to a loaded asset in a [SourceMap].
///
#[derive(Debug, Clone, Copy)]
pub struct Asset<'a> {
    source_map: &'a SourceMap,
    asset: &'a MappedAssetDescriptor,
}

impl<'a> Asset<'a> {
    ///
    /// Creates a new raw asset by referencing a source_map and an asset.
    /// This should only be done when absolutily nessecary.
    ///
    pub fn new(source_map: &'a SourceMap, asset: &'a MappedAssetDescriptor) -> Self {
        Self { source_map, asset }
    }

    ///
    /// Returns the source code as slice of the [SourceMap] buffer.
    ///
    pub fn source(&self) -> &'a str {
        &self.source_map.buffer[self.asset.pos..(self.asset.pos + self.asset.len)]
    }

    ///
    /// Returns the position of the first character of the asset
    /// in the [SourceMap] buffer.
    ///
    pub fn start_pos(&self) -> usize {
        self.asset.pos
    }

    ///
    /// Returns an insert location for a token at the start
    /// of the given asset.
    ///
    pub fn start_loc(&self) -> Loc {
        Loc::new(self.start_pos(), 1, 1)
    }

    ///
    /// Returns the position of the first character of the next asset
    /// in the [SourceMap] buffer.
    ///
    pub fn end_pos(&self) -> usize {
        self.asset.pos + self.asset.len
    }

    ///
    /// Returns an insert location for a token at the end
    /// of the given asset.
    ///
    pub fn end_loc(&self) -> Loc {
        Loc::new(self.end_pos(), 0, self.asset.len_lines)
    }

    ///
    /// Returns the length (bytes) of the current asset.
    ///
    pub fn len_bytes(&self) -> usize {
        self.asset.len
    }

    ///
    /// Returns the number of characters in the asset.
    ///
    pub fn len_chars(&self) -> usize {
        self.source().chars().count()
    }

    ///
    /// Returns the used reference to the [SourceMap].
    ///
    pub fn source_map(&self) -> &'a SourceMap {
        self.source_map
    }

    ///
    /// Returns the used reference to the [MappedAssetDescriptor].
    ///
    pub(crate) fn mapped_asset(&self) -> &'a MappedAssetDescriptor {
        self.asset
    }

    ///
    /// Returns a new [AssetDescriptor] describing the current asset.
    ///
    pub fn descriptor(&self) -> AssetDescriptor {
        AssetDescriptor {
            path: self.asset.path.clone(),
            alias: self.asset.alias.clone(),
        }
    }

    ///
    /// Shortcut to [referenced_slice_for()](SourceMap::referenced_slice_for)
    ///
    pub fn referenced_slice_for(&self, loc: Loc) -> &str {
        self.source_map.referenced_slice_for(loc)
    }

    ///
    /// Shortcut to [padded_referenced_slice_for()](SourceMap::padded_referenced_slice_for)
    ///
    pub fn padded_referenced_slice_for(&self, loc: Loc) -> &str {
        self.source_map
            .padded_referenced_slice_for_with_asset(loc, self.asset)
    }
}
