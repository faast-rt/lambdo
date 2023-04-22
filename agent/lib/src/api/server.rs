use std::{net::IpAddr, str::FromStr, sync::Arc};

use log::{debug, error, info, trace};
use shared::{FileModel, RequestData, RequestStep};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{config::AgentConfig, runner_engine};

use super::{
    client::Client,
    grpc_definitions::{
        lambdo_agent_service_server::LambdoAgentService, Empty, ExecuteRequest, ExecuteResponse,
        StatusMessage,
    },
};

pub struct LambdoAgentServer {
    pub config: AgentConfig,
    pub client: Arc<Mutex<Client>>,
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

        Self {
            config,
            client: Arc::new(Mutex::new(client)),
            id,
        }
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
        info!("Received request execution request");

        // Safe unwrap because proto defines the field as required
        let request = request.into_inner().data.unwrap();
        let request_data = RequestData {
            id: request.id.clone(),
            files: request
                .files
                .iter()
                .map(|f| FileModel {
                    filename: f.filename.clone(),
                    content: f.content.clone(),
                })
                .collect(),
            steps: request
                .steps
                .iter()
                .map(|s| RequestStep {
                    command: s.command.clone(),
                    enable_output: s.enable_output,
                })
                .collect(),
        };
        debug!("Received request: {:?}", request_data);

        let mut runner_engine = runner_engine::service::RunnerEngine::new(request_data);
        let mut self_client = self.client.lock().await;

        if let Err(e) = runner_engine.create_workspace() {
            error!("Failed to create workspace: {}", e);
            self_client
                .status(self.id.clone(), super::grpc_definitions::Code::Error)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to send error status to gRPC server: {}", e);
                    panic!("Failed to send error status to gRPC server");
                });
            return Err(Status::internal("Failed to create workspace"));
        };

        match runner_engine.run() {
            Ok(response) => {
                debug!("Response from runner engine: {:?}", response);

                Ok(Response::new(ExecuteResponse {
                    id: self.id.clone(),
                    steps: response
                        .data
                        .steps
                        .iter()
                        .map(|s| crate::api::grpc_definitions::ExecuteResponseStep {
                            command: s.command.clone(),
                            stderr: s.stderr.clone(),
                            stdout: s.stdout.clone().unwrap(),
                            exit_code: s.exit_code,
                        })
                        .collect(),
                }))
            }
            Err(e) => {
                error!("Failed to run request: {}", e);
                self_client
                    .status(self.id.clone(), super::grpc_definitions::Code::Error)
                    .await
                    .unwrap_or_else(|e| {
                        error!("Failed to send error status to gRPC server: {}", e);
                        panic!("Failed to send error status to gRPC server");
                    });
                Err(Status::internal("Failed to run request"))
            }
        }
    }
}
