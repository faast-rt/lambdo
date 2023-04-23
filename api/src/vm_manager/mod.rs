pub mod state;
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
    pub async fn new(state: LambdoStateRef) -> Result<Self, anyhow::Error> {
        let vmm_manager = VMManager { state };
        vmm_manager.setup_bridge().await?;
        Ok(vmm_manager)
    }

    pub async fn run_code(
        &self,
        request: ExecuteRequest,
        language_settings: LanguageSettings,
    ) -> Result<ExecuteResponse, Error> {
        let ip = self.find_available_ip().await.map_err(|e| {
            error!("Error while finding available IP address: {:?}", e);
            Error::NoIPAvalaible
        })?;

        let mut state = self.state.lock().await;
        let config = &state.config;
        // Safe since we checked the validity of the address before
        let host_ip = Ipv4Inet::from_str(&config.api.bridge_address).unwrap();
        let tap_name = format!("tap-{}", request.id[0..8].to_string());

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

        trace!("Creating channel");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let id = request.id.clone();

        trace!("Creating VMState");
        let mut vm_state = VMState::new(id.clone(), opts.clone(), request.clone(), tx);

        info!(
            "Starting execution for {:?}, (language: {}, version: {})",
            id, language_settings.name, language_settings.version
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
        drop(state);

        info!("Waiting for a connection from VMM {}", id);
        rx.recv().await.ok_or(Error::VmNotFound)?;
        debug!("Received message");

        let mut state = self.state.lock().await;
        let vm = if let Some(vm) = state.vms.iter_mut().find(|vm| vm.id == id) {
            vm
        } else {
            error!("VM not found");
            return Err(Error::VmNotFound);
        };

        if let VMStatus::Ended = vm.state {
            error!("VM already ended");
            return Err(Error::VmAlreadyEnded);
        }
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

        Ok(response)
    }
}
