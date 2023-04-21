use crate::{
    config::LambdoConfig,
    model::{RunRequest, RunResponse},
    vmm::VMMOpts,
};

pub struct LambdoState {
    pub vms: Vec<VMState>,
    pub config: LambdoConfig,
}

pub struct VMState {
    pub id: String,
    pub state: VMStateEnum,
    pub vm_opts: VMMOpts,
    pub request: RunRequest,
    pub response: Option<RunResponse>,
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
            timestamp: std::time::Instant::now(),
            channel,
        }
    }
}

pub enum VMStateEnum {
    Waiting,
    Ready,
    RequestSent,
    ResponseReceived,
    Ended,
}
