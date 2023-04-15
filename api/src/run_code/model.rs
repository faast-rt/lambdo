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

#[derive(Serialize, Deserialize)]
pub struct AgentExecution {
    pub r#type: String,
    pub code: String,
    pub data: AgentExecutionData,
}

#[derive(Serialize, Deserialize)]
pub struct AgentExecutionData {
    pub id: String,
    pub steps: Vec<AgentExecutionStep>,
}

#[derive(Serialize, Deserialize)]
pub struct AgentExecutionStep {
    pub command: String,
    pub result: Option<u8>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub enable_output: bool,
}
