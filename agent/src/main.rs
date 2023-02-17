use agent_lib::{external_api::ExternalApi, internal_api::service::InternalApi};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut internal_api = InternalApi::new("/bin/sh".to_string(), "'Hello world'".to_string());
    let code_return = internal_api.run().map_err(|_| std::fmt::Error)?;
    println!("Stdout: {:?}", code_return.stdout);
    Ok(())
}
