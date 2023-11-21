use std::fs::File;

use anyhow::anyhow;
use clap::Parser;
use env_logger::Env;
use log::{debug, info};

use crate::registry::Registry;

mod httpclient;
mod image;
mod registry;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    image: String,

    #[arg(long, default_value = "./init")]
    init: String,

    #[arg(long, default_value = "./agent")]
    agent: String,

    #[arg(long, default_value = "./config.yaml")]
    agent_config: String,

    #[arg(
        short,
        long,
        default_value = "https://auth.docker.io/token",
        value_name = "auth"
    )]
    auth_url: String,

    #[arg(
        short,
        long,
        default_value = "https://registry-1.docker.io/v2",
        value_name = "registry"
    )]
    registry_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse the cli arguments
    let args = Args::parse();
    debug!("Running cli with arguments : {:?}", args);

    let mut registry = Registry::new(&args.registry_url, &args.auth_url);

    info!("Downloading image {}", &args.image);
    let image = registry.get_image(&args.image).await?;
    info!("Download done!");

    info!("Writing  to disk ...");
    image
        .export_to_initramfs::<File>(&args.init, &args.agent, &args.agent_config)
        .map_err(|e| anyhow!(e).context("Failed to write filesystem to disk"))?;
    info!("Writing done!");

    Ok(())
}
