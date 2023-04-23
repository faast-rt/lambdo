use serde::Deserialize;
use serde::Serialize;

use crate::grpc_definitions::FileModel;

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
