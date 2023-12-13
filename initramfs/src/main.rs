//! This is a tool to transform a container image into an initramfs image.
//! It will download the image from a container registry, extract it and
//! write it to disk.
//! The initramfs image will contain the init binary/script and the agent 
//! binary as well as the agent configuration file.
//! The init binary will be the entrypoint of the initramfs image and will
//! start the agent binary.
//!
//! This image can then be used as an initramfs image for a linux kernel
//! in the lambdo runtime.

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

    let registry = Registry::new(&args.registry_url, &args.auth_url);

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
