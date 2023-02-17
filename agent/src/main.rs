use agent_lib::external_api::ExternalApi;
use env_logger;
use log::info;
use std::error::Error;
use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting agent");

    let mut external_api =
        ExternalApi::new("/dev/pts/4".to_string(), "/dev/pts/5".to_string(), 9600);

    external_api.read_from_serial()?;

    info!("Stopping agent");
    Ok(())
}
