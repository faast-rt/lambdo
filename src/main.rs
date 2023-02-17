use clap::Parser;
use log::info;

#[derive(Parser)]
#[clap(
    version = "0.1",
    author = "Polytech Montpellier - DevOps",
    about = "A Serverless runtime in Rust"
)]
pub struct LambdoOpts {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    LambdoOpts::parse();

    info!("starting up");

    // todo: do something

    info!("shutting down");
    Ok(())
}
