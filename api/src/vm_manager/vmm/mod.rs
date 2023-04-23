pub mod grpc_definitions;
pub mod grpc_server;

use std::{error::Error as STDError, fmt::Display};

use cidr::IpInet;
use lumper::VMM;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub enum Error {
    VmmNew(lumper::Error),
    VmmConfigure(lumper::Error),
    VmmRun(lumper::Error),
    NetSetupError(anyhow::Error),
    BadAgentStatus,
    NoIPAvalaible,
    VmNotFound,
    VmAlreadyEnded,
    GrpcError,
}

impl STDError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::VmmNew(e) => write!(f, "Error while creating VMM: {:?}", e),
            Error::VmmConfigure(e) => write!(f, "Error while configuring VMM: {:?}", e),
            Error::VmmRun(e) => write!(f, "Error while running VMM: {:?}", e),
            Error::NetSetupError(e) => write!(f, "Error while setting up network: {:?}", e),
            Error::BadAgentStatus => write!(f, "Bad agent status"),
            Error::NoIPAvalaible => write!(f, "No IP address available"),
            Error::VmNotFound => write!(f, "VM not found"),
            Error::VmAlreadyEnded => write!(f, "VM already ended"),
            Error::GrpcError => write!(f, "GRPC error"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VMMOpts {
    /// Linux kernel path
    pub kernel: String,
    /// Number of virtual CPUs assigned to the guest
    pub cpus: u8,
    /// Memory amount (in MBytes) assigned to the guest
    pub memory: u32,
    /// Stdout console file path
    pub console: Option<String>,
    /// Path to the socket used for communication with the VMM
    pub socket: Option<String>,
    /// initramfs path
    pub initramfs: Option<String>,
    // Tap interface name
    pub tap: Option<String>,
    // IP address
    pub ip: Option<IpInet>,
    // Gateway
    pub gateway: Option<String>,
}

pub fn run(opts: VMMOpts) -> Result<JoinHandle<Result<(), Error>>, Error> {
    let mut vmm = VMM::new().map_err(Error::VmmNew)?;
    let tap_name = opts.tap.clone();
    vmm.configure(
        opts.cpus,
        opts.memory,
        &opts.kernel,
        opts.console,
        opts.initramfs,
        tap_name,
        opts.socket,
        true,
        Some(opts.ip.unwrap().to_string()),
        opts.gateway,
    )
    .map_err(Error::VmmConfigure)?;

    Ok(tokio::task::spawn_blocking(move || {
        vmm.run(true).map_err(Error::VmmRun)
    }))
}
