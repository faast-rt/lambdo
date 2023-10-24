pub mod state;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use tokio::process::Command;

pub use vmm::grpc_definitions;
pub use vmm::grpc_server::VMListener;
pub use vmm::Error;

use anyhow::anyhow;

use log::{debug, error, info, trace, warn};
use std::{net::IpAddr, str::FromStr};

use crate::{model::LanguageSettings, vm_manager::state::VMStatus};

use self::{
    grpc_definitions::{ExecuteRequest, ExecuteResponse},
    state::LambdoStateRef,
    vmm::run_vm,
};

mod vmm;

pub struct VMManager {
    pub state: LambdoStateRef,
}

impl VMManager {
    pub async fn new(state: LambdoStateRef) -> Result<Self, Error> {
        let mut vmm_manager = VMManager { state };

        {
            let mut state = vmm_manager.state.lock().await;
            setup_bridge(&state).await.map_err(|e| {
                error!("Error while setting up bridge: {:?}", e);
                Error::NetSetupError(e)
            })?;
            let languages = state.config.languages.clone();

            for language_settings in &languages {
                run_vm(&mut state, &language_settings.clone().into(), false)
                    .await
                    .map_err(|e| {
                        error!("Error while setting up language: {:?}", e);
                        e
                    })?;
            }
        }
        vmm_manager.event_listener().await;

        Ok(vmm_manager)
    }

    pub async fn run_code(
        &self,
        request: ExecuteRequest,
        language_settings: LanguageSettings,
    ) -> Result<ExecuteResponse, Error> {
        let mut state = self.state.lock().await;

        debug!("Looking for VM with language: {}", language_settings.name);
        let vm = if let Some(vm) = state.vms.iter_mut().find(|vm| {
            vm.language_settings.name == language_settings.name
                && vm.language_settings.version == language_settings.version
                && !vm.reserved
                && vm.get_state() == VMStatus::Ready
        }) {
            debug!("Found VM {}", vm.id);
            vm
        } else {
            debug!("No VM found, creating one");
            let mut rx = state.channel.1.resubscribe();
            let id = run_vm(&mut state, &language_settings, true)
                .await
                .map_err(|e| {
                    error!("Error while running VM: {:?}", e);
                    e
                })?;

            info!("Waiting for a connection from VMM {}", id);

            drop(state);
            let received_id = loop {
                let r_id = rx.recv().await.map_err(|e| {
                    error!("Error while waiting for VM to start: {:?}", e);
                    Error::VmNotFound
                })?;
                if id != r_id.0 {
                    debug!(
                        "Received message from another VM ({} vs {}), ignoring",
                        id, r_id.0
                    );
                } else {
                    break r_id.0;
                }
            };

            state = self.state.lock().await;
            state
                .vms
                .iter_mut()
                .find(|vm| vm.id == received_id)
                .unwrap()
        };

        if let VMStatus::Ended = vm.get_state() {
            error!("VM already ended");
            return Err(Error::VmAlreadyEnded);
        }

        let response = vm.execute(request).await?;

        Ok(response)
    }

    pub async fn event_listener(&mut self) {
        let mut receiver = self.state.lock().await.channel.1.resubscribe();
        let state = self.state.clone();
        tokio::task::spawn(async move {
            loop {
                match receiver.recv().await {
                    Err(e) => {
                        error!("Error while receiving from channel: {:?}", e);
                        break;
                    }
                    Ok((id, VMStatus::Running)) => {
                        let mut state = state.lock().await;
                        let vm = match state.vms.iter().find(|vm| vm.id == id) {
                            Some(vm) if !vm.reserved => vm,
                            Some(_) => {
                                warn!("VM {} is reserved, ignoring", id);
                                continue;
                            }
                            None => {
                                warn!("VM {} not found while listening to channel", id);
                                continue;
                            }
                        };
                        let language_settings = vm.language_settings.clone();
                        info!("Warming up new VM for language {}", language_settings.name);
                        if let Err(e) = run_vm(&mut state, &language_settings, false).await {
                            error!("Error while running VM: {:?}", e);
                        }
                    }
                    Ok(_) => {}
                }
            }
        });
    }
}

async fn setup_bridge(state: &state::LambdoState) -> anyhow::Result<()> {
    let config = &state.config;
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
    if addresses.iter().any(|addr| {
        addr.ip() == bridge_address.address()
            && addr.netmask() == Some(IpAddr::V4(bridge_address.mask()))
    }) {
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
            .args([
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
            .await
            .map_err(|e| anyhow!("error when adding bridge address: {}", e))?;
    }

    debug!("bringing up bridge");

    Command::new("ip")
        .args(["link", "set", bridge_name, "up"])
        .output()
        .await
        .map_err(|e| anyhow!("error when bringing up bridge: {}", e))?;

    info!("bridge {} is ready", bridge_name);
    Ok(())
}
