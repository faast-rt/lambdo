use std::sync::Arc;

use log::{debug, error, trace};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{
    grpc_definitions::{
        lambdo_api_service_server::LambdoApiService, Empty, RegisterRequest, RegisterResponse,
        StatusMessage,
    },
    state::VMState,
    LambdoState,
};

pub struct VMHandler {
    lambdo_state: Arc<Mutex<LambdoState>>,
}

impl VMHandler {
    pub fn new(lambdo_state: Arc<Mutex<LambdoState>>) -> Self {
        VMHandler { lambdo_state }
    }
}

#[tonic::async_trait]
impl LambdoApiService for VMHandler {
    async fn status(&self, request: Request<StatusMessage>) -> Result<Response<Empty>, Status> {
        unimplemented!()
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
        debug!(
            "Received status request from {}",
            request.remote_addr().unwrap().ip()
        );

        let mut lambdo_state = self.lambdo_state.lock().await;

        let vm = lambdo_state
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
