use crate::{
    config::LambdoLanguageConfig,
    net,
    state::{VMState, VMStateEnum},
    vmm::{self, run, Error, VMMOpts},
    LambdoState,
};
use actix_web::web;
use cidr::Ipv4Inet;
use log::{debug, error, info, trace, warn};
use shared::{RequestData, RequestMessage, ResponseMessage};
use std::{os::unix::net::UnixListener, path::Path, str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::model::RunRequest;

pub async fn run_code(
    state: web::Data<Arc<Mutex<LambdoState>>>,
    request: web::Json<RunRequest>,
) -> Result<ResponseMessage, Error> {
    let entrypoint = request.code[0].filename.clone();
    let socket_name = format!("/tmp/{}.sock", Uuid::new_v4());

    let socket_path = Path::new(socket_name.as_str());
    if std::fs::metadata(socket_path).is_ok() {
        std::fs::remove_file(socket_path).unwrap();
    }

    let mut state = state.lock().await;
    let config = state.config.clone();

    let language_settings =
        find_language(request.language.clone(), config.languages.clone()).unwrap();
    let steps = generate_steps(language_settings.clone(), entrypoint.to_string());
    let file = shared::FileModel {
        filename: entrypoint,
        content: request.code[0].content.clone(),
    };
    let input_filename = "input.input";

    let input = shared::FileModel {
        filename: input_filename.to_string(),
        content: request.input.clone(),
    };

    let request_data = RequestData {
        id: Uuid::new_v4().to_string(),
        steps,
        files: vec![file, input],
    };

    // Safe since we checked the validity of the address before
    let host_ip = Ipv4Inet::from_str(&config.api.bridge_address).unwrap();

    let ip = net::find_available_ip(
        &host_ip,
        &state
            .vms
            .iter()
            .filter_map(|vm| {
                debug!("VM {:?} has ip {:?}", vm.id, vm.vm_opts.ip);
                if vm.vm_opts.ip.is_some() && !matches!(vm.state, VMStateEnum::Ended) {
                    // Safe since we created the address safely
                    Some(
                        Ipv4Inet::from_str(vm.vm_opts.ip.as_ref().unwrap())
                            .unwrap()
                            .address(),
                    )
                } else {
                    warn!("VM {:?} has no IP address", vm.id);
                    None
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
        socket: Some(socket_name.clone()),
        initramfs: Some(language_settings.initramfs.clone()),
        tap: Some(format!("tap-{}", request_data.id[0..8].to_string())),
        ip: Some(ip.to_string()),
        gateway: Some(host_ip.address().to_string()),
    };

    let request_message = RequestMessage {
        r#type: shared::Type::Request,
        code: shared::Code::Run,
        data: request_data,
    };

    trace!("Creating channel");
    let channel = tokio::sync::mpsc::unbounded_channel();

    trace!("Creating VMState");
    let vm_state = VMState::new(
        request_message.data.id.clone(),
        opts.clone(),
        request.into_inner(),
        channel.0,
    );

    state.vms.push(vm_state);
    drop(state);

    info!(
        "Starting execution for {:?}, (language: {}, version: {})",
        request_message.data.id, language_settings.name, language_settings.version
    );
    debug!("Launching VMM with options: {:?}", opts);
    trace!("Request message to VMM: {:?}", request_message);

    let unix_listener = UnixListener::bind(socket_path).unwrap();
    let listener_handler = vmm::listen(unix_listener, request_message);
    run(opts)?;
    let response = listener_handler.join().unwrap();

    match std::fs::remove_file(socket_path) {
        Ok(_) => {}
        Err(e) => {
            warn!(
                "Unable to close socket {} : {}",
                socket_path.to_string_lossy(),
                e
            );
        }
    }

    Ok(parse_response(response))
}

fn parse_response(response: String) -> ResponseMessage {
    // remove first 8 characters of response
    let json = &response[8..];
    let response_message: ResponseMessage = serde_json::from_str(json).unwrap();
    response_message
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
