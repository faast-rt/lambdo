use std::fmt::Debug;

use crate::{config::LambdoConfig, model::LanguageSettings};

use super::{
    grpc_definitions::{
        lambdo_agent_service_client::LambdoAgentServiceClient, ExecuteRequest, ExecuteResponse,
    },
    vmm::VMMOpts,
};

pub type LambdoStateRef = std::sync::Arc<tokio::sync::Mutex<LambdoState>>;

pub struct LambdoState {
    pub vms: Vec<VMState>,
    pub config: LambdoConfig,
    pub channel: (
        tokio::sync::broadcast::Sender<String>,
        tokio::sync::broadcast::Receiver<String>,
    ),
}

impl LambdoState {
    pub fn new(config: LambdoConfig) -> Self {
        let (sender, receiver) = tokio::sync::broadcast::channel(128);
        LambdoState {
            vms: Vec::new(),
            config,
            channel: (sender, receiver),
        }
    }
}

#[derive(Debug)]
pub struct VMState {
    pub id: String,
    pub state: VMStatus,
    pub vm_task: Option<tokio::task::JoinHandle<Result<(), super::vmm::Error>>>,
    pub vm_opts: VMMOpts,
    pub language_settings: LanguageSettings,
    pub request: Option<ExecuteRequest>,
    pub response: Option<ExecuteResponse>,
    pub remote_port: Option<u16>,
    pub client: Option<LambdoAgentServiceClient<tonic::transport::Channel>>,
    pub timestamp: std::time::Instant,
    pub reserved: bool,
}

impl VMState {
    pub fn new(
        id: String,
        vm_opts: VMMOpts,
        language_config: LanguageSettings,
        reserved: bool,
    ) -> Self {
        VMState {
            id,
            state: VMStatus::Waiting,
            vm_task: None,
            vm_opts,
            language_settings: language_config,
            request: None,
            response: None,
            remote_port: None,
            client: None,
            timestamp: std::time::Instant::now(),
            reserved,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMStatus {
    Waiting,
    Ready,
    RequestSent,
    ResponseReceived,
    Ended,
}
