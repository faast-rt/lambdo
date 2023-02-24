use agent_lib::{
    config::AgentConfig, external_api::service::ExternalApi, internal_api::service::InternalApi,
};
use anyhow::{anyhow, Result};
use clap::Parser;
use log::{debug, error, info, trace};

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

    let code_entry = external_api.read_from_serial()?;
    let mut internal_api = InternalApi::new(code_entry);
    internal_api.create_workspace()?;
    let res = internal_api.run().map_err(|e| anyhow!("{:?}", e));

    match res {
        Err(e) => error!("Error: {:?}", e),
        Ok(code) => {
            info!("Code: {:?}", code);

            // Convert Code object to JSON
            let code_json = serde_json::to_string(&code).unwrap();

            // Write the JSON to the serial port
            external_api.write_to_serial(&code_json)?;
        }
    }

    info!("Stopping agent");
    Ok(())
}
