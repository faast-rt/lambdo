use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

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
        AgentExecution {
            r#type: "request".to_string(),
            code: "run".to_string(),
            data: AgentExecutionData {
                id: Uuid::new_v4().to_string(),
                steps: Vec::new(),
                files: Vec::new(),
            },
        }
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
