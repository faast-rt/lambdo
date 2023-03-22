use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct FileModel {
    pub filename: String,
    pub content: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CodeEntry {
    pub files: Vec<FileModel>,
    pub script: Vec<String>, // All commands to execute at startup
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Type {
    Status,
    Request,
    Response,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Code {
    Run,
    Ok,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StatusMessage {
    pub r#type: Type,
    pub code: Code,
}

impl StatusMessage {
    pub fn new() -> StatusMessage {
        StatusMessage {
            r#type: Type::Status,
            code: Code::Ok,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseStep {
    pub command: String,
    pub result: i32,
    pub stdout: String,
    pub stderr: String,
    pub enable_output: bool,
}

impl ResponseStep {
    pub fn new(
        command: String,
        result: i32,
        stdout: String,
        stderr: String,
        enable_output: bool,
    ) -> ResponseStep {
        ResponseStep {
            command,
            result,
            stdout,
            stderr,
            enable_output,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseData {
    pub id: String,
    pub steps: Vec<ResponseStep>,
}

impl ResponseData {
    pub fn new(id: String, steps: Vec<ResponseStep>) -> ResponseData {
        ResponseData { id, steps }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseMessage {
    pub r#type: Type,
    pub code: Code,
    pub data: ResponseData,
}

impl ResponseMessage {
    pub fn new(data: ResponseData) -> ResponseMessage {
        ResponseMessage {
            r#type: Type::Response,
            code: Code::Run,
            data,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RequestStep {
    pub command: String,
    pub enable_output: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RequestData {
    pub id: String,
    pub files: Vec<FileModel>,
    pub steps: Vec<RequestStep>,
}

impl RequestData {
    pub fn new(id: String, files: Vec<FileModel>, steps: Vec<RequestStep>) -> RequestData {
        RequestData { id, files, steps }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RequestMessage {
    pub r#type: Type,
    pub code: Code,
    pub data: RequestData,
}

impl RequestMessage {
    pub fn new(data: RequestData) -> RequestMessage {
        RequestMessage {
            r#type: Type::Request,
            code: Code::Run,
            data,
        }
    }
}
