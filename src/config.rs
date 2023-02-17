use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, BufReader},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LambdoConfigError {
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
pub struct LambdoConfig {
    /// The api version of the lambdo config file
    apiVersion: String,
    /// The kind of the lambdo config file
    kind: String,
    /// The lambdo vmm configuration
    vmm: LambdoVMMConfig,
    /// The lambdo api configuration
    api: LambdoApiConfig,
    /// The lambdo agent configuration
    agent: LambdoAgentConfig,
    /// The lambdo languages configuration
    languages: Vec<LambdoLanguageConfig>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LambdoVMMConfig {
    /// The kernel path to use for the vmm
    kernel: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LambdoApiConfig {
    /// The port on which the API server will listen
    host: String,
    /// The host on which the API server will listen
    port: u16,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LambdoAgentConfig {
    /// The path to the agent binary
    path: String,
    /// The path to the agent configuration file
    config: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LambdoLanguageConfig {
    /// The name of the language
    name: String,
    /// The version of the language
    version: String,
    /// The initramfs path to use for the language
    initramfs: String,
    /// The steps to execute
    steps: Vec<LambdoLanguageStepConfig>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LambdoLanguageStepConfig {
    /// The name of the step
    name: Option<String>,
    /// The command to execute
    command: String,
    /// The output configuration
    output: LambdoLanguageStepOutputConfig,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LambdoLanguageStepOutputConfig {
    /// Is the output enabled ?
    enabled: bool,
    /// Is the output a debug output ?
    debug: bool,
}

impl LambdoConfig {
    /// Load a LambdoConfig from a file.
    ///
    /// Arguments:
    ///
    /// * `path`: The path to the config file.
    ///
    /// Returns:
    ///
    /// A Result<LambdoConfig>
    pub fn load(path: &str) -> Result<Self> {
        let file = File::open(path).map_err(LambdoConfigError::Load)?;
        let reader = BufReader::new(file);
        let config: LambdoConfig =
            serde_yaml::from_reader(reader).map_err(LambdoConfigError::Parse)?;

        if config.kind != "Config" {
            return Err(LambdoConfigError::KindNotSupported.into());
        }

        if config.apiVersion != "lambdo.io/v1alpha1" {
            return Err(LambdoConfigError::VersionNotSupported.into());
        }

        Ok(config)
    }
}
