use std::{net::IpAddr, str::FromStr};

use log::{error, info, trace};
use tonic::{Request, Response, Status};

use crate::config::AgentConfig;

use super::{
    client::Client,
    grpc_definitions::{
        lambdo_agent_service_server::LambdoAgentService, Empty, ExecuteRequest, ExecuteResponse,
        StatusMessage,
    },
};

pub struct LambdoAgentServer {
    pub config: AgentConfig,
    pub client: Client,
    pub id: String,
}

impl LambdoAgentServer {
    pub async fn new(config: AgentConfig) -> Self {
        let grpc_remote_host = IpAddr::from_str(&config.grpc.remote_host).unwrap_or_else(|e| {
            error!("Invalid IP address: {}", config.grpc.remote_host);
            panic!("{}", e.to_string())
        });
        trace!("gRPC remote host: {}", grpc_remote_host);

        trace!("Creating gRPC client..");
        let mut client = Client::new(grpc_remote_host, config.grpc.remote_port).await;

        trace!("Registering to gRPC server..");
        let id = {
            let mut counter = 1;
            loop {
                match client.register(config.grpc.local_port).await {
                    Ok(id) => break id,
                    Err(e) => {
                        error!("Failed to register to gRPC server, {} try: {}", counter, e);
                        counter += 1;
                        if counter >= 10 {
                            panic!("Failed to register to gRPC server");
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        };

        info!("Agent registered with ID: {}", id);

        // Sending ready status right away, since we already opened the listening TCP socket
        info!("Sending ready status to gRPC server..");
        client
            .status(id.clone(), super::grpc_definitions::Code::Ready)
            .await
            .unwrap_or_else(|e| {
                error!("Failed to send ready status to gRPC server: {}", e);
                panic!("Failed to send ready status to gRPC server");
            });

        Self { config, client, id }
    }
}

#[tonic::async_trait]
impl LambdoAgentService for LambdoAgentServer {
    async fn status(&self, request: Request<Empty>) -> Result<Response<StatusMessage>, Status> {
        Err(Status::unimplemented("Not implemented yet"))
    }

    async fn execute(
        &self,
        request: Request<ExecuteRequest>,
    ) -> Result<Response<ExecuteResponse>, Status> {
        Err(Status::unimplemented("Not implemented yet"))
    }
}
