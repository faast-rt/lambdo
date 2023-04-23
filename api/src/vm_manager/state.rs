use std::fmt::Debug;

use crate::config::LambdoConfig;

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
}

impl LambdoState {
    pub fn new(config: LambdoConfig) -> Self {
        LambdoState {
            vms: Vec::new(),
            config,
        }
    }
}

#[derive(Debug)]
pub struct VMState {
    pub id: String,
    pub state: VMStatus,
    pub vm_task: Option<tokio::task::JoinHandle<Result<(), super::vmm::Error>>>,
    pub vm_opts: VMMOpts,
    pub request: ExecuteRequest,
    pub response: Option<ExecuteResponse>,
    pub remote_port: Option<u16>,
    pub client: Option<LambdoAgentServiceClient<tonic::transport::Channel>>,
    pub timestamp: std::time::Instant,
    pub channel: tokio::sync::mpsc::UnboundedSender<bool>,
}

impl VMState {
    pub fn new(
        id: String,
        vm_opts: VMMOpts,
        request: ExecuteRequest,
        channel: tokio::sync::mpsc::UnboundedSender<bool>,
    ) -> Self {
        VMState {
            id,
            state: VMStatus::Waiting,
            vm_task: None,
            vm_opts,
            request,
            response: None,
            remote_port: None,
            client: None,
            timestamp: std::time::Instant::now(),
            channel,
        }
    }
}

#[derive(Debug)]
pub enum VMStatus {
    Waiting,
    Ready,
    RequestSent,
    ResponseReceived,
    Ended,
}
