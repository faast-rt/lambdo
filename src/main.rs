use clap::Parser;
use lambdo::config::LambdoConfig;
use log::{debug, info, trace};

#[derive(Parser)]
#[clap(
    version = "0.1",
    author = "Polytech Montpellier - DevOps",
    about = "A Serverless runtime in Rust"
)]
pub struct LambdoOpts {
    /// Config file path
    #[clap(short, long, default_value = "/etc/lambdo/config.yaml")]
    config: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let options = LambdoOpts::parse();

    info!("starting up ...");

    debug!("loading config file at {}", options.config);
    let config = LambdoConfig::load(options.config.as_str())?;
    trace!(
        "config file loaded successfully with content: {:#?}",
        config
    );

    // todo: do something

    info!("shutting down");
    Ok(())
}
