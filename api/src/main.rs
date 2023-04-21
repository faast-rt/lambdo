pub mod config;
pub mod controller;
pub mod model;
pub mod service;
pub mod vmm;

use config::LambdoConfig;
use thiserror::Error;

use actix_web::{web, App, HttpServer};
use clap::Parser;
use log::{debug, info, trace};
use std::sync::{Arc, Mutex};

use crate::controller::run;

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
pub struct LambdoState {
    vms: Arc<Mutex<Vec<usize>>>,
    config: LambdoConfig,
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

    let host = config.api.host.clone();
    let port = config.api.port.clone();

    info!("Starting server on {}:{}", host, port);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(LambdoState {
                vms: Arc::new(Mutex::new(Vec::new())),
                config: config.clone(),
            }))
            .service(run)
    })
    .bind((host, port))?
    .run()
    .await
}
