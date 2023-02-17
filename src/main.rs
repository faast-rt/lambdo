use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("starting up");

    // todo: do something

    info!("shutting down");
    Ok(())
}
