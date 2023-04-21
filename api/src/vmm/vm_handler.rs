use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{
    grpc_definitions::{
        lambdo_service_server::LambdoService, RequestMessage, ResponseMessage, StatusMessage,
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
impl LambdoService for VMHandler {
    async fn status(
        &self,
        request: Request<StatusMessage>,
    ) -> Result<Response<StatusMessage>, Status> {
        unimplemented!()
    }

    async fn request(
        &self,
        request: Request<RequestMessage>,
    ) -> Result<Response<ResponseMessage>, Status> {
        unimplemented!()
    }
}
