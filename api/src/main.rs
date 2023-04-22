use actix_web::{web, App, HttpServer};
use api::{config::LambdoConfig, controller::run, net::setup_bridge};
use clap::Parser;
use log::{debug, info, trace};

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
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .service(run)
    })
    .bind((host, port))?
    .run()
    .await
}
