use agent_lib::{api::service::Api, config::AgentConfig, runner_engine::service::RunnerEngine};
use anyhow::Result;
use clap::Parser;
use log::{debug, error, info, trace};

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

#[tokio::main]
async fn main() -> Result<()> {
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

    trace!("getting default gateway ip address");
    let gateway = default_net::get_default_gateway().unwrap_or_else(|e| {
        error!("Failed to get default gateway ip address");
        panic!("{}", e.to_string())
    });
    debug!("default gateway ip address: {}", gateway.ip_addr);

    // Initialize API
    let mut api = Api::new(config.serial.path, config.serial.baud_rate, gateway.ip_addr).await;

    // Send status message to serial port
    api.send_status_message().await?;

    // Read request message from serial port
    let request_message = match api.read_from_serial() {
        Ok(request_message) => request_message,
        Err(error) => {
            api.send_error_message(error.to_string()).await?;
            return Ok(());
        }
    };

    let mut runner_engine = RunnerEngine::new(request_message);
    runner_engine.create_workspace()?;
    let response_message = runner_engine.run();
    if let Err(error) = response_message {
        api.send_error_message(error.to_string()).await?;
    } else {
        let response_message = response_message.unwrap();
        api.send_response_message(response_message).await?;
    }

    info!("Stopping agent");
    Ok(())
}
