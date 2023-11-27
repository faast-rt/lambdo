use std::net::IpAddr;

use anyhow::Result;
use grpc_definitions::Code;

pub mod client;
#[rustfmt::skip]
pub mod grpc_definitions;
pub mod server;

/// Client trait
/// This trait is used to abstract the gRPC client
#[tonic::async_trait]
pub trait ClientTrait: Send + Sync {
    async fn status(&mut self, id: String, code: Code) -> Result<()>;
    async fn register(&mut self, port: u16) -> Result<String>;
}

/// SelfCreatingClientTrait trait
/// This trait is used to abstract the gRPC client creation
#[tonic::async_trait]
pub trait SelfCreatingClientTrait: ClientTrait {
    async fn new(grpc_host: IpAddr, port: u16) -> Self;
}
