use std::fmt::Debug;

use crate::{
    config::LambdoConfig,
    grpc_definitions::lambdo_agent_service_client::LambdoAgentServiceClient,
    model::{RunRequest, RunResponse},
    vmm::VMMOpts,
};

pub struct LambdoState {
    pub vms: Vec<VMState>,
    pub config: LambdoConfig,
}

#[derive(Debug)]
pub struct VMState {
    pub id: String,
    pub state: VMStateEnum,
    pub vm_opts: VMMOpts,
    pub request: RunRequest,
    pub response: Option<RunResponse>,
    pub remote_port: Option<u16>,
    pub client: Option<LambdoAgentServiceClient<tonic::transport::Channel>>,
    pub timestamp: std::time::Instant,
    pub channel: tokio::sync::mpsc::UnboundedSender<()>,
}

impl VMState {
    pub fn new(
        id: String,
        vm_opts: VMMOpts,
        request: RunRequest,
        channel: tokio::sync::mpsc::UnboundedSender<()>,
    ) -> Self {
        VMState {
            id,
            state: VMStateEnum::Waiting,
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
pub enum VMStateEnum {
    Waiting,
    Ready,
    RequestSent,
    ResponseReceived,
    Ended,
}
