use anyhow::Result;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, BufReader},
};
use thiserror::Error;

const fn default_remote_port() -> u16 {
    50051
}

const fn default_local_port() -> u16 {
    0
}

fn default_workspace_path() -> String {
    std::env::temp_dir().to_str().unwrap().to_string()
}

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
    /// The gRPC configuration
    #[serde(default = "default_grpc")]
    pub grpc: GRPCConfig,
    /// The workspace where the agent will store the files of Requests and their resulting files
    #[serde(default = "default_workspace_path")]
    pub workspace_path: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GRPCConfig {
    /// The remote gRPC port
    #[serde(default = "default_remote_port")]
    pub remote_port: u16,
    /// The remote gRPC host
    #[serde(default = "default_gateway_ip")]
    pub remote_host: String,
    /// The local gRPC port
    #[serde(default = "default_local_port")]
    pub local_port: u16,
    /// The local gRPC host
    #[serde(default = "default_local_host")]
    pub local_host: String,
}

fn default_local_host() -> String {
    "0.0.0.0".to_string()
}

fn default_gateway_ip() -> String {
    trace!("getting default gateway ip address");
    let gateway = default_net::get_default_gateway().unwrap_or_else(|e| {
        error!("Failed to get default gateway ip address");
        panic!("{}", e)
    });
    debug!("using default gateway ip address: {}", gateway.ip_addr);
    gateway.ip_addr.to_string()
}

fn default_grpc() -> GRPCConfig {
    GRPCConfig {
        remote_port: default_remote_port(),
        remote_host: default_gateway_ip(),
        local_port: default_local_port(),
        local_host: default_local_host(),
    }
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
