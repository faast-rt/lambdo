pub mod grpc_definitions;
pub mod grpc_server;
mod net;

use std::{error::Error as STDError, fmt::Display, str::FromStr};

use cidr::{IpInet, Ipv4Inet};
use log::{debug, error, info, trace};
use lumper::VMM;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{model::LanguageSettings, vm_manager::state::VMState};

use super::state::LambdoState;

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
    ExecutionError,
    Timeout,
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
            Error::ExecutionError => write!(f, "Execution error"),
            Error::Timeout => write!(f, "Timeout"),
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

pub async fn run_vm(
    state: &mut LambdoState,
    language_settings: &LanguageSettings,
    reserved: bool,
) -> Result<String, Error> {
    let ip = net::find_available_ip(state).await.map_err(|e| {
        error!("Error while finding available IP address: {:?}", e);
        Error::NoIPAvalaible
    })?;
    let uuid = Uuid::new_v4().to_string();

    let config = &state.config;
    // Safe since we checked the validity of the address before
    let host_ip = Ipv4Inet::from_str(&config.api.bridge_address).unwrap();
    let tap_name = format!("tap-{}", uuid[0..8].to_string());

    let opts: VMMOpts = VMMOpts {
        kernel: config.vmm.kernel.clone(),
        cpus: 1,
        memory: 1024,
        console: None,
        socket: None,
        initramfs: Some(language_settings.initramfs.clone()),
        tap: Some(tap_name.clone()),
        ip: Some(IpInet::V4(ip)),
        gateway: Some(host_ip.address().to_string()),
    };

    trace!("Creating VMState");
    let mut vm_state = VMState::new(
        uuid.clone(),
        opts.clone(),
        language_settings.clone(),
        state.channel.0.clone(),
        reserved,
    );

    info!(
        "Starting execution for {:?}, (language: {}, version: {})",
        &uuid, language_settings.name, language_settings.version
    );
    debug!("Launching VMM with options: {:?}", opts);
    vm_state.vm_task = Some(run(opts)?);

    debug!("Adding interface to bridge");
    net::add_interface_to_bridge(&tap_name, &*state).map_err(|e| {
        error!("Error while adding interface to bridge: {:?}", e);
        Error::NoIPAvalaible
    })?;
    state.vms.push(vm_state);

    Ok(uuid.clone())
}
