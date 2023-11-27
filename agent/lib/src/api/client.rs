use std::net::IpAddr;

use anyhow::{anyhow, Result};
use log::{error, info, trace};

use crate::api::grpc_definitions::{register_response::Response, RegisterRequest};

use super::{
    grpc_definitions::{lambdo_api_service_client::LambdoApiServiceClient, Code, StatusMessage},
    ClientTrait, SelfCreatingClientTrait,
};

pub struct Client {
    client: LambdoApiServiceClient<tonic::transport::Channel>,
}

#[tonic::async_trait]
impl SelfCreatingClientTrait for Client {
    async fn new(gprc_host: IpAddr, port: u16) -> Self {
        info!("Connecting to gRPC server at {}:{}", gprc_host, port);

        let mut counter = 0;
        while counter < 10 {
            match LambdoApiServiceClient::connect(format!("http://{}:{}", gprc_host, port)).await {
                Ok(client) => {
                    info!("Connected to gRPC server at {}:{}", gprc_host, port);
                    return Self { client };
                }
                Err(e) => {
                    error!("Failed to connect to gRPC server: {}", e);
                    counter += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }

        panic!("Failed to connect to gRPC server");
    }
}

#[tonic::async_trait]
impl ClientTrait for Client {
    async fn register(&mut self, port: u16) -> Result<String> {
        info!("Registering to lambdo..");
        let register_response = self
            .client
            .register(RegisterRequest { port: port.into() })
            .await?;
        trace!("Register response: {:?}", register_response);

        match register_response.into_inner().response.unwrap() {
            Response::Error(error) => Err(anyhow!("Error registering with gRPC server: {}", error)),
            Response::Id(id) => Ok(id),
        }
    }

    async fn status(&mut self, id: String, code: Code) -> Result<()> {
        self.client
            .status(StatusMessage {
                id,
                code: code.into(),
            })
            .await
            .map_or_else(|e| Err(anyhow!("Error sending status: {}", e)), |_| Ok(()))
    }
}
