use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, BufReader},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentConfigError {
    #[error("cannot load config file")]
    Load(#[from] io::Error),
    #[error("cannot parse config file")]
    Parse(#[from] serde_yaml::Error),
    #[error("unsupported config kind")]
    KindNotSupported,
    #[error("unsupported config api version")]
    VersionNotSupported,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[allow(non_snake_case)]
pub struct AgentConfig {
    /// The api version of the agent config file
    pub apiVersion: String,
    /// The kind of the agent config file
    pub kind: String,
    /// The serial configuration
    pub serial: SerialConfig,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct SerialConfig {
    /// The path to the serial port
    pub path: String,
    /// The baud rate to use for the serial port
    pub baud_rate: u32,
}

impl AgentConfig {
    /// Load a AgentConfig from a file.
    ///
    /// Arguments:
    ///
    /// * `path`: The path to the config file.
    ///
    /// Returns:
    ///
    /// A Result<AgentConfig>
    pub fn load(path: &str) -> Result<Self> {
        let file = File::open(path).map_err(AgentConfigError::Load)?;
        let reader = BufReader::new(file);
        let config: AgentConfig =
            serde_yaml::from_reader(reader).map_err(AgentConfigError::Parse)?;

        if config.kind != "AgentConfig" {
            return Err(AgentConfigError::KindNotSupported.into());
        }

        if config.apiVersion != "lambdo.io/v1alpha1" {
            return Err(AgentConfigError::VersionNotSupported.into());
        }

        Ok(config)
    }
}
