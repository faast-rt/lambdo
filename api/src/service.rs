use crate::{
    config::{LambdoConfig, LambdoLanguageConfig},
    vmm::{self, run, Error, VMMOpts},
};
use actix_web::web;
use log::warn;
use shared::{RequestData, RequestMessage, ResponseMessage};
use std::{os::unix::net::UnixListener, path::Path};
use uuid::Uuid;

use crate::model::RunRequest;

pub fn run_code(
    config: web::Data<LambdoConfig>,
    request: web::Json<RunRequest>,
) -> Result<ResponseMessage, Error> {
    let entrypoint = request.code[0].filename.clone();
    let socket_name = format!("/tmp/{}.sock", Uuid::new_v4().to_string());

    let socket_path = Path::new(socket_name.as_str());
    if std::fs::metadata(socket_path).is_ok() {
        std::fs::remove_file(socket_path).unwrap();
    }

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

    let request_message = RequestMessage {
        r#type: shared::Type::Request,
        code: shared::Code::Run,
        data: request_data,
    };

    let opts: VMMOpts = VMMOpts {
        kernel: config.vmm.kernel.clone(),
        cpus: 1,
        memory: 1024,
        console: None,
        socket: Some(socket_name.clone()),
        initramfs: Some(language_settings.initramfs.clone()),
    };

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
