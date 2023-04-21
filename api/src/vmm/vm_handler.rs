use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{
    grpc_definitions::{
        lambdo_api_service_server::LambdoApiService, Empty, RegisterRequest, RegisterResponse,
        StatusMessage,
    },
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
        unimplemented!()
    }
}
