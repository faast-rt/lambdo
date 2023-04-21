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

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[allow(non_snake_case)]
pub struct LambdoConfig {
    /// The api version of the lambdo config file
    pub apiVersion: String,
    /// The kind of the lambdo config file
    pub kind: String,
    /// The lambdo vmm configuration
    pub vmm: LambdoVMMConfig,
    /// The lambdo api configuration
    pub api: LambdoApiConfig,
    /// The lambdo agent configuration
    pub agent: LambdoAgentConfig,
    /// The lambdo languages configuration
    pub languages: Vec<LambdoLanguageConfig>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LambdoVMMConfig {
    /// The kernel path to use for the vmm
    pub kernel: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LambdoApiConfig {
    /// The host on which the API server will listen
    pub web_host: String,
    /// The port on which the API server will listen
    pub web_port: u16,
    /// The host on which the gRPC server will listen
    pub grpc_host: String,
    /// The port on which the gRPC server will listen
    pub gprc_port: u16,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LambdoAgentConfig {
    /// The path to the agent binary
    pub path: String,
    /// The path to the agent configuration file
    pub config: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LambdoLanguageConfig {
    /// The name of the language
    pub name: String,
    /// The version of the language
    pub version: String,
    /// The initramfs path to use for the language
    pub initramfs: String,
    /// The steps to execute
    pub steps: Vec<LambdoLanguageStepConfig>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LambdoLanguageStepConfig {
    /// The name of the step
    pub name: Option<String>,
    /// The command to execute
    pub command: String,
    /// The output configuration
    pub output: LambdoLanguageStepOutputConfig,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct LambdoLanguageStepOutputConfig {
    /// Is the output enabled ?
    pub enabled: bool,
    /// Is the output a debug output ?
    pub debug: bool,
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
