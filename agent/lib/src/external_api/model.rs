use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct FileModel {
    pub filename: String,
    pub content: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CodeEntry {
    pub files: Vec<FileModel>,
    pub script: Vec<String>,
}
