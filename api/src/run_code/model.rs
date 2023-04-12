use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize)]
pub struct File {
    pub filename: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct RunRequest {
    pub language: String,
    pub version: String,
    pub input: String,
    pub code: Vec<File>,
}


#[derive(Serialize)]
pub struct RunResponse {
    pub status: u8,
    pub stdout: String,
    pub stderr: String,
}