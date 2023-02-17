use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct FileModel {
    fileName: String,
    fileContent: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CodeEntry {
    files: Vec<FileModel>,
    script: Vec<String>,
}
