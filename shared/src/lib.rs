use serde::{Deserialize, Serialize};

/// Represents a file to be included in the workspace
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct FileModel {
    /// Name of the file, paths relative to the workspace
    pub filename: String,
    /// Content of the file
    pub content: String,
}

/// Identifies the type of the message
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum Type {
    /// Status message to indicate that the agent is ready
    #[serde(rename = "status")]
    Status,
    /// Request message
    #[serde(rename = "request")]
    Request,
    /// Response message answering to a request message
    #[serde(rename = "response")]
    Response,
}

/// Code to tell what the Request/Response message is about
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum Code {
    /// Represents a request to run the code or a response to such request
    #[serde(rename = "run")]
    Run,
    /// Agent is ready to communicate
    #[serde(rename = "ready")]
    Ready,
}

/// Represents a Status message
#[derive(Deserialize, Serialize, Debug)]
pub struct StatusMessage {
    /// Type of the message
    pub r#type: Type,
    /// Code of the message
    pub code: Code,
}

impl StatusMessage {
    pub fn new(code: Code) -> StatusMessage {
        StatusMessage {
            // r#type is a reserved keyword in Rust, so we need to use the raw identifier syntax
            r#type: Type::Status,
            code,
        }
    }
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self::new(Code::Ready)
    }
}

/// Serializes an Option<String> as a String by returning an empty string if the Option is None
fn serialize_optionnal_string<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(v) => serializer.serialize_str(v),
        None => serializer.serialize_str(""),
    }
}

/// Represents the output of a step
#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseStep {
    /// Command that was run
    pub command: String,
    /// Exit code of the command
    #[serde(alias = "exitCode")]
    pub exit_code: i32,
    /// Stdout of the command. If it is None, it will be serialized as an empty string
    /// to avoid api crashes
    #[serde(serialize_with = "serialize_optionnal_string")]
    pub stdout: Option<String>,
    /// Stderr of the command
    pub stderr: String,
}

impl ResponseStep {
    pub fn new(
        command: String,
        exit_code: i32,
        stdout: Option<String>,
        stderr: String,
    ) -> ResponseStep {
        ResponseStep {
            command,
            exit_code,
            stdout,
            stderr,
        }
    }
}

/// Contains the id of the request and the result of all steps
#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseData {
    /// Id of the request (UUID)
    pub id: String,
    /// Result of all steps
    pub steps: Vec<ResponseStep>,
}

impl ResponseData {
    pub fn new(id: String, steps: Vec<ResponseStep>) -> ResponseData {
        ResponseData { id, steps }
    }
}

/// Represents a Response message with code Type::Run, meaning that it is a response to a run code request
#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseMessage {
    ///  Type of the message
    pub r#type: Type,
    /// Code of the message
    pub code: Code,
    /// Data of the message
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

/// Represent a step in the request with type Type::Run
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RequestStep {
    /// Command to run
    pub command: String,
    /// Whether the stdout should be returned or not (stderr will alaways be)
    #[serde(alias = "enableOutput")]
    pub enable_output: bool,
}

/// Represents the data of a request message with type Type::Run
#[derive(Deserialize, Serialize, Debug)]
pub struct RequestData {
    /// Id of the request (UUID)
    pub id: String,
    /// Files to be included in the workspace, paths relative to the workspace
    pub files: Vec<FileModel>,
    /// Steps to be executed
    pub steps: Vec<RequestStep>,
}

impl RequestData {
    pub fn new(id: String, files: Vec<FileModel>, steps: Vec<RequestStep>) -> RequestData {
        RequestData { id, files, steps }
    }
}

/// Represents a Request message with type Type::Run
#[derive(Deserialize, Serialize, Debug)]
pub struct RequestMessage {
    /// Type of the message
    pub r#type: Type,
    /// Code of the message
    pub code: Code,
    /// Data of the message
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
