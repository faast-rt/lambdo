use std::sync::Arc;

use anyhow::anyhow;
use log::{debug, error, info, trace};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{
    grpc_definitions::{
        self, lambdo_agent_service_client::LambdoAgentServiceClient,
        lambdo_api_service_server::LambdoApiService, Empty, RegisterRequest, RegisterResponse,
        StatusMessage,
    },
    state::{VMState, VMStateEnum},
    LambdoState,
};

pub struct VMHandler {
    lambdo_state: Arc<Mutex<LambdoState>>,
}

impl VMHandler {
    pub fn new(lambdo_state: Arc<Mutex<LambdoState>>) -> Self {
        VMHandler { lambdo_state }
    }

    pub async fn vm_ready(&self, vm: &mut VMState) -> Result<(), anyhow::Error> {
        debug!("VM {} is ready", vm.id);
        vm.state = VMStateEnum::Ready;

        // Safe, since we created the VMState with an IP
        let ip = vm.vm_opts.ip.unwrap().address();
        // Safe, since we got the port already at this stage
        let address = format!("http://{}:{}", ip, vm.remote_port.unwrap());

        info!(
            "Trying to connect to VM {}, using address {}",
            vm.id, address
        );
        let client = LambdoAgentServiceClient::connect(address)
            .await
            .map_err(|e| anyhow!("Failed to connect to VM {}: {}", vm.id, e))?;

        info!("Connected to VM {}", vm.id);
        vm.client = Some(client);
        Ok(())
    }
}

#[tonic::async_trait]
impl LambdoApiService for VMHandler {
    async fn status(&self, request: Request<StatusMessage>) -> Result<Response<Empty>, Status> {
        let request = request.into_inner();
        debug!("Received status request: {:#?}", request);

        let mut lambdo_state = self.lambdo_state.lock().await;

        let vm = match lambdo_state.vms.iter_mut().find(|vm| vm.id.eq(&request.id)) {
            Some(vm) => vm,
            None => {
                error!("No VM found for this ID: {}", request.id);
                return Err(Status::not_found("No VM found for this ID"));
            }
        };
        debug!("VM {} send a status", vm.id);

        match request.code() {
            grpc_definitions::Code::Ready => self.vm_ready(vm).await.unwrap_or_else(|e| {
                error!("Failed to handle VM ready status: {}", e);
                vm.state = VMStateEnum::Ended;
            }),
            grpc_definitions::Code::Error => {
                error!("VM {} reported an error", vm.id);
                // TODO: Better error handling
                vm.channel.send(()).unwrap();
                vm.state = VMStateEnum::Ended;
            }
            grpc_definitions::Code::Run => {
                info!("VM {} send sent a Run status", vm.id);
            }
        };
        debug!("Sending empty status response");

        Ok(Response::new(Empty {}))
    }

    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        trace!("Received register request");
        if request.remote_addr().is_none() {
            error!("No remote address");
            return Ok(Response::new(RegisterResponse {
                response: Some(crate::grpc_definitions::register_response::Response::Error(
                    "No remote address".to_string(),
                )),
            }));
        }
        info!(
            "Received register request from {}",
            request.remote_addr().unwrap().ip()
        );

        let mut lambdo_state = self.lambdo_state.lock().await;

        let mut vm = lambdo_state
            .vms
            .iter_mut()
            .filter(|vm| match vm.vm_opts.ip {
                Some(ip) if ip.address().eq(&request.remote_addr().unwrap().ip()) => true,
                _ => false,
            })
            .collect::<Vec<&mut VMState>>();

        match vm.len() {
            0 => {
                error!(
                    "No VM found for this IP: {}",
                    request.remote_addr().unwrap().ip()
                );
                return Ok(Response::new(RegisterResponse {
                    response: Some(crate::grpc_definitions::register_response::Response::Error(
                        "No VM found for this IP".to_string(),
                    )),
                }));
            }
            1 => {
                vm[0].remote_port = Some(request.into_inner().port.try_into().unwrap());
                Ok(Response::new(RegisterResponse {
                    response: Some(crate::grpc_definitions::register_response::Response::Id(
                        vm[0].id.clone(),
                    )),
                }))
            }
            _ => {
                error!(
                    "Multiple VM found for this IP: {}",
                    request.remote_addr().unwrap().ip()
                );
                error!("VMs: {:#?}", vm);
                return Ok(Response::new(RegisterResponse {
                    response: Some(crate::grpc_definitions::register_response::Response::Error(
                        "Multiple VM found for this IP".to_string(),
                    )),
                }));
            }
        }
    }
}
