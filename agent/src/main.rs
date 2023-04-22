use agent_lib::{
    api::{
        grpc_definitions::lambdo_agent_service_server::LambdoAgentServiceServer,
        server::LambdoAgentServer,
    },
    config::AgentConfig,
};
use anyhow::Result;
use clap::Parser;
use log::{debug, error, info, trace};
use tokio::net::TcpListener;

/// Agent CLI options
#[derive(Parser)]
#[clap(
    version = "0.1",
    author = "Polytech Montpellier - DevOps",
    about = "A Serverless runtime in Rust"
)]
pub struct AgentOpts {
    /// Config file path
    #[clap(short, long, default_value = "/etc/lambdo/agent/config.yaml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    info!("Starting agent");

    // Parse CLI options
    let options = AgentOpts::parse();
    debug!("loading config file at {}", options.config);

    // Load config file
    let mut config = AgentConfig::load(options.config.as_str())?;
    trace!("config loaded successfully with content: {:#?}", config);

    // Initialize gRPC server
    let tcp_socket = TcpListener::bind(format!(
        "{}:{}",
        config.grpc.local_host, config.grpc.local_port
    ))
    .await
    .unwrap_or_else(|e| {
        error!("Failed to bind to port {}", config.grpc.local_port);
        panic!("{}", e.to_string())
    });

    config.grpc.local_port = tcp_socket.local_addr().unwrap().port();
    info!(
        "gRPC server listening on {}:{}",
        config.grpc.local_host, config.grpc.local_port
    );
    let tcp_stream = tokio_stream::wrappers::TcpListenerStream::new(tcp_socket);

    tonic::transport::Server::builder()
        .add_service(LambdoAgentServiceServer::new(
            LambdoAgentServer::new(config).await,
        ))
        .serve_with_incoming(tcp_stream)
        .await
        .unwrap_or_else(|e| {
            error!("GRPC Server failure");
            panic!("{}", e)
        });

    info!("Stopping agent");
    Ok(())
}
