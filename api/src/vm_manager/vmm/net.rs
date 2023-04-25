use std::net::IpAddr;
use std::process::Command;
use std::str::FromStr;

use anyhow::anyhow;
use anyhow::Result;
use cidr::IpInet;
use cidr::Ipv4Inet;
use log::{debug, error, info, trace};
use network_interface::NetworkInterface;
use network_interface::NetworkInterfaceConfig;

use crate::vm_manager::state::LambdoState;
use crate::vm_manager::state::LambdoStateRef;
use crate::vm_manager::state::VMStatus;

pub(super) fn add_interface_to_bridge(interface_name: &String, state: &LambdoState) -> Result<()> {
    let bridge_name = &state.config.api.bridge;
    debug!(
        "adding interface {} to bridge {}",
        interface_name, bridge_name
    );

    trace!("fetching interface id");
    let interface_id = network_bridge::interface_id(interface_name)
        .map_err(|e| anyhow!("error when fetching interface id: {}", e))?;

    trace!("interface id: {}", interface_id);
    network_bridge::add_interface_to_bridge(interface_id, bridge_name)
        .map_err(|e| anyhow!("error when adding interface to bridge: {}", e))?;

    debug!("bringing up interface");
    Command::new("ip")
        .args(&["link", "set", interface_name, "up"])
        .output()
        .map_err(|e| anyhow!("error when bringing up interface: {}", e))?;

    info!(
        "interface {} added to bridge {}",
        interface_name, bridge_name
    );
    Ok(())
}

pub(super) async fn find_available_ip(state: &LambdoState) -> Result<Ipv4Inet> {
    let config = &state.config;
    // Safe since we checked the validity of the address before
    let host_ip = Ipv4Inet::from_str(&config.api.bridge_address).unwrap();

    let used_ip: &Vec<_> = &state
        .vms
        .iter()
        .filter_map(|vm| {
            debug!("VM {:?} has ip {:?}", vm.id, vm.vm_opts.ip);
            match vm.vm_opts.ip {
                Some(IpInet::V4(ip)) if vm.get_state() != VMStatus::Ended => Some(ip.address()),
                _ => None,
            }
        })
        .collect();
    debug!("looking for available ip in {}", host_ip);
    trace!("used ip: {:?}", used_ip);
    let mut ip = host_ip.clone();
    ip.increment();

    while used_ip.contains(&ip.address()) {
        trace!("ip {} is already used, trying next one", ip);
        if ip.increment() {
            return Err(anyhow!("no available ip"));
        }
    }

    info!("found available ip: {}", ip);
    return Ok(ip);
}
