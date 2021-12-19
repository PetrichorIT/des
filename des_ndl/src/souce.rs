use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAssetDescriptor {
    pub path: PathBuf,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAsset {
    pub descriptor: SourceAssetDescriptor,
    pub data: String,
}

impl SourceAsset {
    pub fn load(path: PathBuf, alias: String) -> std::io::Result<Self> {
        let data = std::fs::read_to_string(&path)?;
        Ok(Self {
            descriptor: SourceAssetDescriptor { path, alias },
            data,
        })
    }
}
