use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Debug)]
pub struct File {
    pub filename: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Default)]
pub struct AgentExecution {
    pub r#type: String,
    pub code: String,
    pub data: AgentExecutionData,
}

#[derive(Serialize, Deserialize, Default)]
pub struct AgentExecutionData {
    pub id: String,
    pub steps: Vec<AgentExecutionStep>,
    pub files: Vec<AgentExecutionFile>,
}

#[derive(Serialize, Deserialize)]
pub struct AgentExecutionStep {
    pub command: String,
    pub enable_output: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AgentExecutionFile {
    pub filename: String,
    pub content: String,
}

impl AgentExecution {
    pub fn new() -> Self {
        AgentExecution::default()
    }

    pub fn add_step(&mut self, step: AgentExecutionStep) {
        self.data.steps.push(step);
    }

    pub fn add_file(&mut self, file: AgentExecutionFile) {
        self.data.files.push(file);
    }
}

impl AgentExecutionStep {
    pub fn new(command: String, enable_output: bool) -> Self {
        AgentExecutionStep {
            command,
            enable_output,
        }
    }
}

impl AgentExecutionFile {
    pub fn new(filename: String, content: String) -> Self {
        AgentExecutionFile { filename, content }
    }
}
