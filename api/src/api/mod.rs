pub mod service;

use actix_web::{post, web, Responder};
use log::{debug, error, info, trace, warn};

use crate::{
    api::service::LambdoApiService,
    model::{RunRequest, RunResponse},
    vm_manager::{self, grpc_definitions::ExecuteResponse},
};
use std::error::Error;

#[post("/run")]
async fn run(
    run_body: web::Json<RunRequest>,
    service: web::Data<LambdoApiService>,
) -> Result<impl Responder, Box<dyn Error>> {
    debug!(
        "Received code execution request from http (language: {}, version: {})",
        run_body.language, run_body.version
    );
    trace!("Request body: {:?}", run_body);

    let response = service.run_code(run_body.into_inner()).await;

    let response = match response {
        Ok(response) => {
            info!("Execution ended for {:?}", response.id);
            trace!("Response: {:?}", response);
            parse_response(response)
        }
        // for the moment just signal an internal server error
        Err(e) => match e {
            vm_manager::Error::Timeout => {
                warn!("Timeout while executing code");
                return Ok(web::Json(RunResponse {
                    status: 128,
                    stdout: "".to_string(),
                    stderr: "Timeout".to_string(),
                }));
            }
            _ => {
                error!("Error while executing code: {:?}", e);
                RunResponse {
                    status: 1,
                    stdout: "".to_string(),
                    stderr: "Internal server error".to_string(),
                }
            }
        },
    };

    Ok(web::Json(response))
}

fn parse_response(response: ExecuteResponse) -> RunResponse {
    let mut stdout = String::new();
    let mut stderr = String::new();
    for step in response.steps.as_slice() {
        if !step.stdout.is_empty() {
            stdout.push_str(step.stdout.as_str());
        }
        stderr.push_str(step.stderr.as_str());
    }

    RunResponse {
        status: response.steps[response.steps.len() - 1]
            .exit_code
            .try_into()
            .unwrap(),
        stdout,
        stderr,
    }
}

#[cfg(test)]
mod test{
    use crate::{vm_manager::grpc_definitions::{ExecuteResponse, ExecuteResponseStep}, api::parse_response};


    #[test]
    fn test_parse_response_stdout() {
        let response = ExecuteResponse {
            id: "test".to_string(),
            steps: vec![
                ExecuteResponseStep {
                    command: "echo Hello".to_string(),
                    stdout: "Hello".to_string(),
                    stderr: "".to_string(),
                    exit_code: 0,
                },
                ExecuteResponseStep {
                    command: "echo World".to_string(),
                    stdout: "World".to_string(),
                    stderr: "".to_string(),
                    exit_code: 0,
                },
            ],
        };

        let parsed = parse_response(response);

        assert_eq!(parsed.stdout, "HelloWorld");
        assert_eq!(parsed.stderr, "");
        assert_eq!(parsed.status, 0);
    }

    #[test]
    fn test_parse_response_with_error() {
        let response = ExecuteResponse {
            id: "test".to_string(),
            steps: vec![
                ExecuteResponseStep {
                    command: "echo Hello".to_string(),
                    stdout: "Hello".to_string(),
                    stderr: "".to_string(),
                    exit_code: 0,
                },
                ExecuteResponseStep {
                    command: "echo World".to_string(),
                    stdout: "".to_string(),
                    stderr: "Error".to_string(),
                    exit_code: 1,
                },
            ],
        };

        let parsed = parse_response(response);

        assert_eq!(parsed.stdout, "Hello");
        assert_eq!(parsed.stderr, "Error");
        assert_eq!(parsed.status, 1);
    }
}