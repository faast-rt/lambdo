pub mod config;
pub mod controller;
pub mod grpc_definitions;
pub mod model;
pub mod service;
pub mod state;
pub mod vmm;

use std::sync::Arc;

use config::LambdoConfig;
use thiserror::Error;

use actix_web::{web, App, HttpServer};
use clap::Parser;
use log::{debug, error, info, trace};
use tokio::sync::Mutex;

use crate::{
    controller::run, grpc_definitions::lambdo_service_server::LambdoServiceServer,
    state::LambdoState, vmm::vm_handler::VMHandler,
};

#[derive(Parser)]
#[clap(
    version = "0.1",
    author = "Polytech Montpellier - DevOps",
    about = "A Serverless runtime in Rust"
)]
pub struct LambdoOpts {
    /// Config file path
    #[clap(short, long, default_value = "/etc/lambdo/config.yaml")]
    config: String,
}

#[derive(Error, Debug)]
pub enum LambdoError {
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    #[error("unknown lambdo error")]
    Unknown,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let options = LambdoOpts::parse();

    info!("starting up ...");

    debug!("loading config file at {}", options.config);
    let config = LambdoConfig::load(options.config.as_str()).unwrap();
    trace!(
        "config file loaded successfully with content: {:#?}",
        config
    );

    let lambdo_state = Arc::new(Mutex::new(LambdoState {
        vms: Vec::new(),
        config: config.clone(),
    }));

    let lambdo_state_clone = lambdo_state.clone();
    // TODO: Shut down the web server when the gRPC server is down, and vice versa
    tokio::spawn(async move {
        let grpc_host = config.api.grpc_host;
        let grpc_port = config.api.gprc_port;
        let addr = format!("{}:{}", grpc_host, grpc_port).parse().unwrap();
        info!("Starting gRPC server on {}", addr);
        let vm_handler = VMHandler::new(lambdo_state_clone);
        tonic::transport::Server::builder()
            .add_service(LambdoServiceServer::new(vm_handler))
            .serve(addr)
            .await
            .unwrap_or_else(|e| {
                error!("GRPC Server failure");
                panic!("{}", e)
            });
    });

    let http_host = config.api.web_host;
    let http_port = config.api.web_port;
    info!("Starting web server on {}:{}", http_host, http_port);
    let app_state = web::Data::new(lambdo_state);
    HttpServer::new(move || App::new().app_data(app_state.clone()).service(run))
        .bind((http_host, http_port))?
        .run()
        .await
}
