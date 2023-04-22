use crate::{
    config::LambdoLanguageConfig,
    grpc_definitions::{ExecuteRequest, ExecuteRequestData, ExecuteResponse},
    net,
    state::{VMState, VMStateEnum},
    vmm::{run, Error, VMMOpts},
    LambdoState,
};
use actix_web::web;
use cidr::{IpInet, Ipv4Inet};
use log::{debug, error, info, trace};
use shared::{RequestData, ResponseData};
use std::{str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::model::RunRequest;

pub async fn run_code(
    state: web::Data<Arc<Mutex<LambdoState>>>,
    request: web::Json<RunRequest>,
) -> Result<ResponseData, Error> {
    let entrypoint = request.code[0].filename.clone();

    let mut lambdo_state = state.lock().await;
    let config = lambdo_state.config.clone();

    let language_settings =
        find_language(request.language.clone(), config.languages.clone()).unwrap();
    let steps = generate_steps(language_settings.clone(), entrypoint.to_string());
    let file = shared::FileModel {
        filename: entrypoint.to_string(),
        content: request.code[0].content.clone(),
    };
    let input_filename = "input.input";

    let input = shared::FileModel {
        filename: input_filename.to_string(),
        content: request.input.clone(),
    };

    let request_data = RequestData {
        id: Uuid::new_v4().to_string(),
        steps: steps,
        files: vec![file, input],
    };
    trace!("Request message to VMM: {:?}", request_data);

    // Safe since we checked the validity of the address before
    let host_ip = Ipv4Inet::from_str(&config.api.bridge_address).unwrap();

    let ip = net::find_available_ip(
        &host_ip,
        &lambdo_state
            .vms
            .iter()
            .filter_map(|vm| {
                debug!("VM {:?} has ip {:?}", vm.id, vm.vm_opts.ip);
                match vm.vm_opts.ip {
                    Some(IpInet::V4(ip)) if !matches!(vm.state, VMStateEnum::Ended) => {
                        Some(ip.address())
                    }
                    _ => None,
                }
            })
            .collect(),
    )
    .map_err(|e| {
        error!("Error while finding available IP address: {:?}", e);
        Error::NoIPAvalaible
    })?;

    let opts: VMMOpts = VMMOpts {
        kernel: config.vmm.kernel.clone(),
        cpus: 1,
        memory: 1024,
        console: None,
        socket: None,
        initramfs: Some(language_settings.initramfs.clone()),
        tap: Some(format!("tap-{}", request_data.id[0..8].to_string())),
        ip: Some(IpInet::V4(ip)),
        gateway: Some(host_ip.address().to_string()),
    };

    trace!("Creating channel");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let id = request_data.id.clone();

    trace!("Creating VMState");
    let mut vm_state = VMState::new(id.clone(), opts.clone(), request_data, tx);

    info!(
        "Starting execution for {:?}, (language: {}, version: {})",
        id, language_settings.name, language_settings.version
    );
    debug!("Launching VMM with options: {:?}", opts);

    vm_state.vm_task = Some(run(opts)?);
    lambdo_state.vms.push(vm_state);
    drop(lambdo_state);

    info!("Waiting for a connection from VMM {}", id);
    rx.recv().await.ok_or(Error::VmNotFound)?;
    debug!("Received message");

    let mut state = state.lock().await;
    let vm = if let Some(vm) = state.vms.iter_mut().find(|vm| vm.id == id) {
        vm
    } else {
        error!("VM not found");
        return Err(Error::VmNotFound);
    };

    if let VMStateEnum::Ended = vm.state {
        error!("VM already ended");
        return Err(Error::VmAlreadyEnded);
    }
    info!("Running payload for {}", vm.id);
    let response = vm
        .client
        .as_mut()
        .unwrap()
        .execute(request_data_into_grpc_request(&vm.request, vm.id.clone()))
        .await
        .map_err(|e| {
            error!("Error while executing request: {:?}", e);
            Error::GrpcError
        })?
        .into_inner();
    debug!("Response from VMM: {:?}", response);

    Ok(execution_response_into_response_message(&response))
}

fn find_language(
    language: String,
    language_list: Vec<LambdoLanguageConfig>,
) -> Result<LambdoLanguageConfig, Box<dyn std::error::Error>> {
    for lang in language_list {
        if lang.name == language {
            return Ok(lang);
        }
    }
    Err("Language not found".into())
}

fn generate_steps(
    language_settings: LambdoLanguageConfig,
    entrypoint: String,
) -> Vec<shared::RequestStep> {
    let mut steps: Vec<shared::RequestStep> = Vec::new();
    for step in language_settings.steps {
        let command = step.command.replace("{{filename}}", entrypoint.as_str());

        steps.push(shared::RequestStep {
            command,
            enable_output: step.output.enabled,
        });
    }
    steps
}

fn request_data_into_grpc_request(request: &RequestData, id: String) -> ExecuteRequest {
    ExecuteRequest {
        id: id.clone(),
        data: Some(ExecuteRequestData {
            id: id.clone(),
            files: request
                .files
                .iter()
                .map(|f| crate::grpc_definitions::FileModel {
                    filename: f.filename.clone(),
                    content: f.content.clone(),
                })
                .collect(),
            steps: request
                .steps
                .iter()
                .map(|s| crate::grpc_definitions::ExecuteRequestStep {
                    command: s.command.clone(),
                    enable_output: s.enable_output.clone(),
                })
                .collect(),
        }),
    }
}

fn execution_response_into_response_message(response: &ExecuteResponse) -> ResponseData {
    // Safe since proto definition guarantees that data is present
    let data = response.data.as_ref().unwrap();
    ResponseData {
        id: data.id.clone(),
        steps: data
            .steps
            .iter()
            .map(|s| shared::ResponseStep {
                command: s.command.clone(),
                stdout: Some(s.stdout.clone()),
                stderr: s.stderr.clone(),
                exit_code: s.exit_code,
            })
            .collect(),
    }
}
