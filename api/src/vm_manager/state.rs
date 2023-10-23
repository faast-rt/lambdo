use std::fmt::Debug;

use anyhow::anyhow;
use log::{debug, error, info, warn};
use tokio::select;

use crate::{config::LambdoConfig, model::LanguageSettings, vm_manager::Error};

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

    #[allow(clippy::type_complexity)]
    pub channel: (
        tokio::sync::broadcast::Sender<(String, VMStatus)>,
        tokio::sync::broadcast::Receiver<(String, VMStatus)>,
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

    pub fn find_ready_vms(&mut self) -> Option<&mut VMState> {
        self.vms
            .iter_mut()
            .find(|vm| vm.get_state() == VMStatus::Ready)
    }
}

#[derive(Debug)]
pub struct VMState {
    pub id: String,
    state: VMStatus,
    pub vm_task: Option<tokio::task::JoinHandle<Result<(), super::vmm::Error>>>,
    pub vm_opts: VMMOpts,
    pub language_settings: LanguageSettings,
    pub request: Option<ExecuteRequest>,
    pub response: Option<ExecuteResponse>,
    remote_port: Option<u16>,
    client: Option<LambdoAgentServiceClient<tonic::transport::Channel>>,

    #[allow(dead_code)]
    start_timestamp: tokio::time::Instant,
    execute_timestamp: Option<tokio::time::Instant>,
    tx: tokio::sync::broadcast::Sender<(String, VMStatus)>,
    pub reserved: bool,
}

impl VMState {
    pub fn new(
        id: String,
        vm_opts: VMMOpts,
        language_config: LanguageSettings,
        tx: tokio::sync::broadcast::Sender<(String, VMStatus)>,
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
            start_timestamp: tokio::time::Instant::now(),
            execute_timestamp: None,
            tx,
            reserved,
        }
    }

    pub fn get_state(&self) -> VMStatus {
        self.state
    }

    pub async fn execute(
        &mut self,
        request: ExecuteRequest,
    ) -> Result<ExecuteResponse, super::vmm::Error> {
        self.request = Some(request.clone());
        self.set_state(VMStatus::Running);

        info!("Running payload on {}", self.id);

        select! {
            response =
            self
            .client
            .as_mut()
            .unwrap()
            .execute(request.clone())
             => {
                let response = response.map_err(|e| {
                    warn!("Error while executing request: {:?}", e);
                    debug!("Request: {:?}", request);
                    self.set_state(VMStatus::Ended);
                    Error::ExecutionError
                })?
                .into_inner();

                self.response = Some(response.clone());
                debug!("Response from VMM: {:?}", response);

                self.set_state(VMStatus::Ended);
                Ok(response)
            }

            _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                warn!("Timeout while executing request");
                self.set_state(VMStatus::Ended);
                Err(Error::Timeout)
            }
        }
    }

    pub async fn ready(&mut self) -> Result<(), anyhow::Error> {
        debug!("VM {} is ready", self.id);
        self.set_state(VMStatus::Ready);

        // Safe, since we created the VMState with an IP
        let ip = self.vm_opts.ip.unwrap().address();
        // Safe, since we got the port already at this stage
        let address = format!("http://{}:{}", ip, self.remote_port.unwrap());

        info!(
            "Trying to connect to VM {}, using address {}",
            self.id, address
        );

        let client = LambdoAgentServiceClient::connect(address)
            .await
            .map_err(|e| {
                self.set_state(VMStatus::Ended);
                anyhow!("Failed to connect to VM {}: {}", self.id, e)
            })?;

        info!("Connected to VM {}", self.id);
        self.client = Some(client);

        Ok(())
    }

    pub fn register(&mut self, port: u32) -> Result<u16, anyhow::Error> {
        match port.try_into() {
            Ok(port) => {
                self.remote_port = Some(port);
                Ok(port)
            }
            Err(e) => {
                error!("Failed to convert port: {}", e);
                self.set_state(VMStatus::Ended);
                Err(anyhow!("Failed to convert port: {}", e))
            }
        }
    }

    fn set_state(&mut self, state: VMStatus) {
        match state {
            VMStatus::Ready => {
                debug!("VM {} is ready", self.id);
                self.state = state;
                self.tx.send((self.id.clone(), self.state)).unwrap();
            }
            VMStatus::Running => {
                debug!("VM {} is running", self.id);
                self.execute_timestamp = Some(tokio::time::Instant::now());
                self.state = state;
                self.tx.send((self.id.clone(), self.state)).unwrap();
            }
            VMStatus::Ended => {
                debug!("VM {} has ended", self.id);
                // TODO: Find a way to kill the VM
                // Probably need to make edit lumper
                self.state = state;
                self.tx.send((self.id.clone(), self.state)).unwrap();
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMStatus {
    Waiting,
    Ready,
    Running,
    Ended,
}
