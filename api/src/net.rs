use std::net::IpAddr;
use std::process::Command;
use std::str::FromStr;

use anyhow::anyhow;
use anyhow::Result;
use log::{debug, error, info, trace};
use network_interface::NetworkInterface;
use network_interface::NetworkInterfaceConfig;

pub fn setup_bridge(bridge_name: &str, bridge_address: &str) -> Result<()> {
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
        .filter(|iface| iface.name == bridge_name)
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

pub fn add_interface_to_bridge(interface_name: &str, bridge_name: &str) -> Result<()> {
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
