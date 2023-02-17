use agent_lib::{external_api::ExternalApi, internal_api::service::InternalApi};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut external_api =
        ExternalApi::new("/dev/pts/4".to_string(), "/dev/pts/5".to_string(), 9600);

    external_api.read_from_serial();

    Ok(())
}
