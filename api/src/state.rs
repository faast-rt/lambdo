use std::fmt::Debug;

use crate::{
    config::LambdoConfig,
    grpc_definitions::{
        lambdo_agent_service_client::LambdoAgentServiceClient, ExecuteRequest, ExecuteResponse,
    },
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
    pub vm_task: Option<tokio::task::JoinHandle<Result<(), crate::vmm::Error>>>,
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
            state: VMStateEnum::Waiting,
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
pub enum VMStateEnum {
    Waiting,
    Ready,
    RequestSent,
    ResponseReceived,
    Ended,
}
