pub mod state;
use uuid::Uuid;
pub use vmm::grpc_definitions;
pub use vmm::grpc_server::VMListener;
pub use vmm::Error;
mod net;

use cidr::{IpInet, Ipv4Inet};
use log::{debug, error, info, trace};
use std::str::FromStr;

use crate::{
    model::LanguageSettings,
    vm_manager::{
        state::{VMState, VMStatus},
        vmm::{run, VMMOpts},
    },
};

use self::{
    grpc_definitions::{ExecuteRequest, ExecuteResponse},
    state::LambdoStateRef,
};

mod vmm;

pub struct VMManager {
    pub state: LambdoStateRef,
}

impl VMManager {
    pub async fn new(state: LambdoStateRef) -> Result<Self, Error> {
        let vmm_manager = VMManager { state };
        vmm_manager.setup_bridge().await.map_err(|e| {
            error!("Error while setting up bridge: {:?}", e);
            Error::NetSetupError(e)
        })?;

        let languages = vmm_manager.state.lock().await.config.languages.clone();

        for language_settings in &languages {
            vmm_manager
                .run_vm(&language_settings.clone().into(), false)
                .await
                .map_err(|e| {
                    error!("Error while setting up language: {:?}", e);
                    e
                })?;
        }

        Ok(vmm_manager)
    }

    pub async fn run_vm(
        &self,
        language_settings: &LanguageSettings,
        reserved: bool,
    ) -> Result<String, Error> {
        let ip = self.find_available_ip().await.map_err(|e| {
            error!("Error while finding available IP address: {:?}", e);
            Error::NoIPAvalaible
        })?;
        let uuid = Uuid::new_v4().to_string();

        let mut state = self.state.lock().await;
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
            reserved,
        );

        info!(
            "Starting execution for {:?}, (language: {}, version: {})",
            &uuid, language_settings.name, language_settings.version
        );
        debug!("Launching VMM with options: {:?}", opts);
        vm_state.vm_task = Some(run(opts)?);

        debug!("Adding interface to bridge");
        self.add_interface_to_bridge(&tap_name, &*state)
            .map_err(|e| {
                error!("Error while adding interface to bridge: {:?}", e);
                Error::NoIPAvalaible
            })?;
        state.vms.push(vm_state);

        Ok(uuid.clone())
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
                && vm.state == VMStatus::Ready
        }) {
            debug!("Found VM {}", vm.id);
            vm
        } else {
            debug!("No VM found, creating one");
            let mut rx = state.channel.1.resubscribe();
            drop(state);
            let id = self.run_vm(&language_settings, true).await.map_err(|e| {
                error!("Error while running VM: {:?}", e);
                e
            })?;

            info!("Waiting for a connection from VMM {}", id);

            let received_id = loop {
                let r_id = rx.recv().await.map_err(|e| {
                    error!("Error while waiting for VM to start: {:?}", e);
                    Error::VmNotFound
                })?;
                if id != r_id {
                    debug!(
                        "Received message from another VM ({} vs {}), ignoring",
                        id, r_id
                    );
                } else {
                    break r_id;
                }
            };

            state = self.state.lock().await;
            state
                .vms
                .iter_mut()
                .find(|vm| vm.id == received_id)
                .unwrap()
        };

        if let VMStatus::Ended = vm.state {
            error!("VM already ended");
            return Err(Error::VmAlreadyEnded);
        }

        vm.request = Some(request.clone());

        vm.state = VMStatus::RequestSent;

        info!("Running payload for {}", vm.id);
        let response = vm
            .client
            .as_mut()
            .unwrap()
            .execute(request)
            .await
            .map_err(|e| {
                error!("Error while executing request: {:?}", e);
                Error::GrpcError
            })?
            .into_inner();

        vm.response = Some(response.clone());
        debug!("Response from VMM: {:?}", response);

        vm.state = VMStatus::Ended;

        Ok(response)
    }
}
