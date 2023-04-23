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

use crate::vm_manager::state::VMStatus;

use super::state::LambdoState;
use super::VMManager;

impl VMManager {
    pub(super) async fn setup_bridge(&self) -> Result<()> {
        let config = &self.state.lock().await.config;
        let bridge_name = &config.api.bridge;
        let bridge_address = &config.api.bridge_address;
        trace!("validating bridge address");
        let bridge_address = cidr::Ipv4Inet::from_str(bridge_address)
            .map_err(|e| anyhow!("invalid bridge address: {}", e))?;
        trace!("bridge address is valid");
        trace!("validating bridge name");
        if bridge_name.len() > 15 {
            return Err(anyhow!("bridge name is too long"));
        }
        trace!("bridge name is valid");

        info!(
            "setting up bridge {} with address {}",
            bridge_name, bridge_address
        );
        let bridge = network_bridge::interface_id(bridge_name)
            .map_or_else(
                |e| {
                    trace!("error when fetching bridge id: {}", e);
                    debug!("bridge {} does not exist, creating it", bridge_name);
                    network_bridge::create_bridge(bridge_name)
                },
                |id| {
                    debug!("bridge {} already exists, using it", bridge_name);
                    Ok(id)
                },
            )
            .map_err(|e| {
                error!("error when creating bridge, am I running as root?");
                anyhow!("error when creating bridge: {}", e)
            })?;

        trace!("bridge id: {}", bridge);
        debug!("looking for existing bridge address");
        let addresses = NetworkInterface::show()
            .map_err(|e| anyhow!("error when fetching network interfaces: {}", e))?
            .into_iter()
            .filter(|iface| iface.name == *bridge_name)
            .flat_map(|iface| iface.addr)
            .collect::<Vec<_>>();

        trace!("existing addresses: {:?}", addresses);
        if addresses
            .iter()
            .find(|addr| {
                addr.ip() == bridge_address.address()
                    && addr.netmask() == Some(IpAddr::V4(bridge_address.mask()))
            })
            .is_some()
        {
            debug!("bridge address already exists, skipping");
        } else {
            debug!("bridge address does not exist, creating it");
            trace!(
                "Values: {} {}/{}",
                bridge_name,
                bridge_address.address(),
                bridge_address.network_length()
            );
            Command::new("ip")
                .args(&[
                    "addr",
                    "add",
                    &format!(
                        "{}/{}",
                        bridge_address.address(),
                        bridge_address.network_length()
                    ),
                    "dev",
                    bridge_name,
                ])
                .output()
                .map_err(|e| anyhow!("error when adding bridge address: {}", e))?;
        }

        debug!("bringing up bridge");

        Command::new("ip")
            .args(&["link", "set", bridge_name, "up"])
            .output()
            .map_err(|e| anyhow!("error when bringing up bridge: {}", e))?;

        info!("bridge {} is ready", bridge_name);
        Ok(())
    }

    pub(crate) fn add_interface_to_bridge(
        &self,
        interface_name: &String,
        state: &LambdoState,
    ) -> Result<()> {
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

    pub(super) async fn find_available_ip(&self) -> Result<Ipv4Inet> {
        let state = self.state.lock().await;
        let config = &state.config;
        // Safe since we checked the validity of the address before
        let host_ip = Ipv4Inet::from_str(&config.api.bridge_address).unwrap();

        let used_ip: &Vec<_> = &state
            .vms
            .iter()
            .filter_map(|vm| {
                debug!("VM {:?} has ip {:?}", vm.id, vm.vm_opts.ip);
                match vm.vm_opts.ip {
                    Some(IpInet::V4(ip)) if vm.state != VMStatus::Ended => Some(ip.address()),
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
}
