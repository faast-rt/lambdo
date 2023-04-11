use agent_lib::{
    config::AgentConfig, external_api::service::ExternalApi, internal_api::service::InternalApi,
};
use anyhow::{anyhow, Result};
use clap::Parser;
use log::{debug, info, trace};

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

/// Main function
fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("Starting agent");

    // Parse CLI options
    let options = AgentOpts::parse();

    debug!("loading config file at {}", options.config);

    // Load config file
    let config = AgentConfig::load(options.config.as_str())?;

    trace!(
        "config file loaded successfully with content: {:#?}",
        config
    );

    // Initialize external API
    let mut external_api = ExternalApi::new(config.serial.path, config.serial.baud_rate);

    // Send status message to serial port
    external_api.send_status_message()?;

    // Read request message from serial port
    let request_message = external_api.read_from_serial()?;

    // Initialize internal API
    let mut internal_api = InternalApi::new(request_message);

    // Create the workspace
    internal_api.create_workspace()?;

    // Run the steps of the request message
    let response_message = internal_api.run().map_err(|e| anyhow!("{:?}", e))?;

    // Send response message to serial port
    external_api.send_response_message(response_message)?;

    info!("Stopping agent");
    Ok(())
}
