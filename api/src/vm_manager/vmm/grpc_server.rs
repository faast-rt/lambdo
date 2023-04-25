use std::sync::Arc;

use super::grpc_definitions::{
    lambdo_api_service_server::LambdoApiService, register_response, Code, Empty, RegisterRequest,
    RegisterResponse, StatusMessage,
};
use log::{debug, error, info, trace};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{
    vm_manager::state::{VMState, VMStatus},
    LambdoState,
};

pub struct VMListener {
    lambdo_state: Arc<Mutex<LambdoState>>,
}

impl VMListener {
    pub fn new(lambdo_state: Arc<Mutex<LambdoState>>) -> Self {
        VMListener { lambdo_state }
    }
}

#[tonic::async_trait]
impl LambdoApiService for VMListener {
    async fn status(&self, request: Request<StatusMessage>) -> Result<Response<Empty>, Status> {
        let request = request.into_inner();
        debug!("Received status request: {:#?}", request);

        let mut lambdo_state = self.lambdo_state.lock().await;
        let tx = lambdo_state.channel.0.clone();

        let vm = match lambdo_state.vms.iter_mut().find(|vm| vm.id.eq(&request.id)) {
            Some(vm) => vm,
            None => {
                error!("No VM found for this ID: {}", request.id);
                return Err(Status::not_found("No VM found for this ID"));
            }
        };
        debug!("VM {} send a status", vm.id);

        match request.code() {
            Code::Ready => vm.ready().await.unwrap_or_else(|e| {
                error!("Failed to handle VM ready status: {}", e);
            }),
            Code::Error => {
                error!("VM {} reported an error", vm.id);
            }
            Code::Run => {
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
                response: Some(register_response::Response::Error(
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
                Some(ip)
                    if ip.address().eq(&request.remote_addr().unwrap().ip())
                        && vm.get_state() != VMStatus::Ended =>
                {
                    true
                }
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
                    response: Some(register_response::Response::Error(
                        "No VM found for this IP".to_string(),
                    )),
                }));
            }
            1 => {
                if let Err(e) = vm[0].register(request.into_inner().port) {
                    error!("Failed to register VM: {}", e);
                    Ok(Response::new(RegisterResponse {
                        response: Some(register_response::Response::Error(
                            "Failed to register VM".to_string(),
                        )),
                    }))
                } else {
                    Ok(Response::new(RegisterResponse {
                        response: Some(register_response::Response::Id(vm[0].id.clone())),
                    }))
                }
            }
            _ => {
                error!(
                    "Multiple VM found for this IP: {}",
                    request.remote_addr().unwrap().ip()
                );
                error!("VMs: {:#?}", vm);
                return Ok(Response::new(RegisterResponse {
                    response: Some(register_response::Response::Error(
                        "Multiple VM found for this IP".to_string(),
                    )),
                }));
            }
        }
    }
}
