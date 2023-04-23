use serde::Deserialize;
use serde::Serialize;

use crate::config::LambdoLanguageConfig;
use crate::vm_manager::grpc_definitions::FileModel;

#[derive(Deserialize, Debug)]
pub struct RunRequest {
    pub language: String,
    pub version: String,
    pub input: String,
    pub code: Vec<FileModel>,
}

#[derive(Serialize, Debug)]
pub struct RunResponse {
    pub status: u8,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct LanguageSettings {
    pub name: String,
    pub version: String,
    pub initramfs: String,
}

impl From<LambdoLanguageConfig> for LanguageSettings {
    fn from(config: LambdoLanguageConfig) -> Self {
        LanguageSettings {
            name: config.name,
            version: config.version,
            initramfs: config.initramfs,
        }
    }
}
