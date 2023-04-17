use anyhow::{anyhow, Result};
use agent_lib::{api::service::Api};
use log::{debug, info, trace};

/// Main function
fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("Starting agent");

    // Initialize API
    let mut api = Api::new("/dev/pts/6".to_string(), 9200);

    // Send status message to serial port
    api.send_status_message()?;

    // Read request message from serial port
    let request_message = api.read_from_serial()?;

    info!("Stopping agent");
    Ok(())
}
