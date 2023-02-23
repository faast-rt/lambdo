use agent_lib::{external_api::service::ExternalApi, internal_api::service::InternalApi};
use anyhow::{ Result, anyhow };

use log::{info, error};

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting agent");

    let mut external_api =
        ExternalApi::new("/dev/pts/4".to_string(), "/dev/pts/6".to_string(), 9600);

    let code_entry = external_api.read_from_serial()?;
    let mut internal_api = InternalApi::new(code_entry);
    internal_api.create_workspace()?;
    let res = internal_api.run().map_err(|e| anyhow!("{:?}", e));

    match res {
        Err(e) => error!("Error: {:?}", e),
        Ok(code) => info!("Code: {:?}", code),
    }

    info!("Stopping agent");
    Ok(())
}
