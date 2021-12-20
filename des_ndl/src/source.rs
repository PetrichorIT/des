use std::path::{Component, PathBuf};

///
/// A descriptor of an asset based on it filepath (relative to WORK_DIR)
/// and its internal alias.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAssetDescriptor {
    /// The path to the given asset.
    pub path: PathBuf,
    /// The simplified call path used in includes.
    pub alias: String,
}

impl SourceAssetDescriptor {
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
    pub fn from_path(path: PathBuf, rel_root: &PathBuf) -> Self {
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
    pub fn from_alias(alias: String, rel_root: &PathBuf) -> Self {
        let mut path = rel_root.clone();

        for split in alias.split("/") {
            path.push(split)
        }
        path.set_extension(".ndl");

        Self { path, alias }
    }
}

///
/// A stored NDL file read for processing (read-only).
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAsset {
    /// The descriptor that defines the location and alias of the file.
    pub descriptor: SourceAssetDescriptor,
    /// A buffer storing the raw data.
    pub data: String,

    // Statistics
    pub lines: usize,
    pub chars: usize,
    pub line_pos_mapping: Vec<usize>,
}

impl SourceAsset {
    ///
    /// Loads a asset using the given descriptor.
    /// This may fail if the read operation fails.
    ///
    pub fn load(descriptor: SourceAssetDescriptor) -> std::io::Result<Self> {
        let data = std::fs::read_to_string(&descriptor.path)?;

        let mut pos = 0;
        let mut lines = 0;
        let mut chars = 0;
        let mut line_pos_mapping = Vec::new();

        line_pos_mapping.push(0); // "ZERO" Line
        line_pos_mapping.push(0); // First Line

        for c in data.chars() {
            if c == '\n' {
                line_pos_mapping.push(pos + 1);
                lines += 1
            }

            pos += 1;
            chars += 1;
        }

        Ok(Self {
            descriptor,
            data,
            lines,
            chars,
            line_pos_mapping,
        })
    }

    pub fn line_of_pos(&self, pos: usize) -> usize {
        for (line, line_start) in self.line_pos_mapping.iter().enumerate() {
            if *line_start > pos {
                // return line;
                return line.saturating_sub(1).max(1);
            }
        }
        return self.lines;
    }
}
