pub mod vm_handler;

use cidr::IpInet;
use log::debug;
use lumper::VMM;
use tokio::task::JoinHandle;

use crate::net;

#[derive(Debug)]
pub enum Error {
    VmmNew(lumper::Error),

    VmmConfigure(lumper::Error),

    VmmRun(lumper::Error),

    BadAgentStatus,

    NoIPAvalaible,

    VmNotFound,

    VmAlreadyEnded,
    GrpcError,
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

    debug!("Adding interface to bridge");
    net::add_interface_to_bridge(&opts.tap.unwrap(), "lambdo0").unwrap();
    Ok(tokio::task::spawn_blocking(move || {
        vmm.run(true).map_err(Error::VmmRun)
    }))
}
