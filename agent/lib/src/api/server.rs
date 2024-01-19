use std::{net::IpAddr, str::FromStr, sync::Arc};

use log::{debug, error, info, trace};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{api::ClientTrait, config::AgentConfig, runner_engine};

use super::{
    grpc_definitions::{
        lambdo_agent_service_server::LambdoAgentService, Empty, ExecuteRequest, ExecuteResponse,
        StatusMessage,
    },
    SelfCreatingClientTrait,
};

pub struct LambdoAgentServer {
    pub config: AgentConfig,
    pub client: Arc<Mutex<Box<dyn ClientTrait>>>,
    pub id: String,
}

impl LambdoAgentServer {
    pub async fn new<C: ClientTrait + SelfCreatingClientTrait + 'static>(
        config: AgentConfig,
    ) -> Self {
        let grpc_remote_host = IpAddr::from_str(&config.grpc.remote_host).unwrap_or_else(|e| {
            error!("Invalid IP address: {}", config.grpc.remote_host);
            panic!("{}", e.to_string())
        });
        trace!("gRPC remote host: {}", grpc_remote_host);

        trace!("Creating gRPC client..");
        let mut client = C::new(grpc_remote_host, config.grpc.remote_port).await;

        trace!("Registering to gRPC server..");
        let id = {
            let mut counter = 1;
            loop {
                match client.register(config.grpc.local_port).await {
                    Ok(id) => break id,
                    Err(e) => {
                        error!("Failed to rese provide us with your discord handle, after joining our servergister to gRPC server, {} try: {}", counter, e);
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
            client: Arc::new(Mutex::new(Box::new(client))),
            id,
        }
    }
}

#[tonic::async_trait]
impl LambdoAgentService for LambdoAgentServer {
    async fn status(&self, _request: Request<Empty>) -> Result<Response<StatusMessage>, Status> {
        Err(Status::unimplemented("Not implemented yet"))
    }

    async fn execute(
        &self,
        request: Request<ExecuteRequest>,
    ) -> Result<Response<ExecuteResponse>, Status> {
        info!("Received request execution request");

        let request = request.into_inner();
        debug!("Received request: {:?}", request);

        let mut runner_engine =
            runner_engine::service::RunnerEngine::new(request, &self.config.workspace_path);
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
                    steps: response.steps,
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

#[cfg(test)]
mod test {
    use super::super::grpc_definitions::Code;
    use crate::{
        api::{
            grpc_definitions::{
                lambdo_agent_service_server::LambdoAgentService, Empty, ExecuteRequest,
                ExecuteRequestStep,
            },
            server::LambdoAgentServer,
            ClientTrait, SelfCreatingClientTrait,
        },
        config::{AgentConfig, GRPCConfig},
    };
    use anyhow::Result;
    use tonic::Request;

    struct MockClient;

    #[tonic::async_trait]
    impl ClientTrait for MockClient {
        async fn register(&mut self, _local_port: u16) -> Result<String> {
            Ok("test".to_string())
        }

        async fn status(&mut self, _id: String, _code: Code) -> Result<()> {
            Ok(())
        }
    }

    #[tonic::async_trait]
    impl SelfCreatingClientTrait for MockClient {
        async fn new(_grpc_host: std::net::IpAddr, _port: u16) -> Self {
            MockClient
        }
    }

    #[tokio::test]
    async fn status_unimplemented() {
        let config = AgentConfig {
            apiVersion: "lambdo.io/v1alpha1".to_string(),
            kind: "AgentConfig".to_string(),
            grpc: GRPCConfig {
                remote_port: 50051,
                remote_host: "127.0.0.1".to_string(),
                local_host: "127.0.0.1".to_string(),
                local_port: 50051,
            },
            workspace_path: tempfile::tempdir()
                .unwrap()
                .into_path()
                .to_str()
                .unwrap()
                .to_string(),
        };

        let server = LambdoAgentServer::new::<MockClient>(config).await;
        let status = server.status(Request::new(Empty {})).await;

        assert!(status.is_err());
    }

    #[tokio::test]
    async fn execute_works() {
        let config = AgentConfig {
            apiVersion: "lambdo.io/v1alpha1".to_string(),
            kind: "AgentConfig".to_string(),
            grpc: GRPCConfig {
                remote_port: 50051,
                remote_host: "127.0.0.1".to_string(),
                local_host: "127.0.0.1".to_string(),
                local_port: 50051,
            },
            workspace_path: tempfile::tempdir()
                .unwrap()
                .into_path()
                .to_str()
                .unwrap()
                .to_string(),
        };

        let server = LambdoAgentServer::new::<MockClient>(config).await;
        let execute = server
            .execute(Request::new(ExecuteRequest {
                id: "test".to_string(),
                files: vec![],
                steps: vec![ExecuteRequestStep {
                    command: "echo -n 'This is stdout' && echo -n 'This is stderr' >&2 && exit 1"
                        .to_string(),
                    enable_output: true,
                }],
            }))
            .await;

        assert!(execute.is_ok());

        let execution_recap = execute.unwrap().into_inner();

        assert_eq!(execution_recap.clone().steps[0].stdout, "This is stdout");
        assert_eq!(execution_recap.steps[0].stderr, "This is stderr");
    }
}
