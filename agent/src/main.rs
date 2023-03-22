use agent_lib::{
    config::AgentConfig, external_api::service::ExternalApi, internal_api::service::InternalApi,
};
use anyhow::{anyhow, Result};
use clap::Parser;
use log::{debug, info, trace};

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

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting agent");

    let options = AgentOpts::parse();

    debug!("loading config file at {}", options.config);
    let config = AgentConfig::load(options.config.as_str())?;

    trace!(
        "config file loaded successfully with content: {:#?}",
        config
    );

    let mut external_api = ExternalApi::new(config.serial.path, config.serial.baud_rate);

    external_api.send_status_message()?;

    let request_message = external_api.read_from_serial()?;
    let mut internal_api = InternalApi::new(request_message);
    internal_api.create_workspace()?;
    let response_message = internal_api.run().map_err(|e| anyhow!("{:?}", e))?;
    external_api.send_response_message(response_message)?;

    info!("Stopping agent");
    Ok(())
}
