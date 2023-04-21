pub mod config;
pub mod controller;
pub mod model;
pub mod net;
pub mod service;
pub mod state;
pub mod vmm;

use std::sync::Arc;

use config::LambdoConfig;
use thiserror::Error;

use actix_web::{web, App, HttpServer};
use clap::Parser;
use log::{debug, info, trace};
use tokio::sync::Mutex;

use crate::{controller::run, state::LambdoState};
use net::setup_bridge;

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

#[actix_web::main]
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

    setup_bridge(&config.api.bridge, &config.api.bridge_address).unwrap();

    let host = config.api.host.clone();
    let port = config.api.port;

    info!("Starting server on {}:{}", host, port);
    let state = web::Data::new(Arc::new(Mutex::new(LambdoState {
        vms: Vec::new(),
        config: config.clone(),
    })));

    HttpServer::new(move || App::new().app_data(state.clone()).service(run))
        .bind((host, port))?
        .run()
        .await
}
