pub mod service;

use actix_web::{post, web, Responder};
use log::{debug, error, info, trace, warn};

use crate::{
    api::service::{LambdoApiService, LambdoApiServiceTrait},
    model::{RunRequest, RunResponse},
    vm_manager::{self, grpc_definitions::ExecuteResponse},
};
use std::error::Error;

async fn run_code(run_resquest: RunRequest, service: &dyn LambdoApiServiceTrait) -> RunResponse {
    let response = service.run_code(run_resquest).await;

    match response {
        Ok(response) => {
            info!("Execution ended for {:?}", response.id);
            trace!("Response: {:?}", response);
            parse_response(response)
        }
        Err(e) => match e {
            vm_manager::Error::Timeout => {
                warn!("Timeout while executing code");
                RunResponse {
                    status: 128,
                    stdout: "".to_string(),
                    stderr: "Timeout".to_string(),
                }
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
    }
}

#[post("/run")]
pub async fn post_run_route(
    run_body: web::Json<RunRequest>,
    api_service: web::Data<LambdoApiService>,
) -> Result<impl Responder, Box<dyn Error>> {
    debug!(
        "Received code execution request from http (language: {}, version: {})",
        run_body.language, run_body.version
    );
    trace!("Request body: {:?}", run_body);

    let service = api_service.get_ref();
    let result = run_code(run_body.into_inner(), service);

    Ok(web::Json(result.await))
}

fn parse_response(response: ExecuteResponse) -> RunResponse {
    if response.steps.is_empty() {
        return RunResponse {
            status: 1,
            stdout: "".to_string(),
            stderr: "Nothing was run".to_string(),
        };
    }

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
            .unwrap_or(1),
        stdout,
        stderr,
    }
}

#[cfg(test)]
mod test {
    use std::vec;

    use crate::{
        api::{parse_response, run_code},
        model::RunRequest,
        vm_manager::grpc_definitions::{ExecuteResponse, ExecuteResponseStep, FileModel},
    };

    use super::service::MockLambdoApiServiceTrait;

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

    #[tokio::test]
    async fn test_run_code_with_no_steps() {
        let mut mock_service = MockLambdoApiServiceTrait::new();
        mock_service.expect_run_code().once().returning(|_| {
            Ok(ExecuteResponse {
                id: "test".to_string(),
                steps: vec![],
            })
        });

        let run_request = RunRequest {
            language: "Node".to_string(),
            version: "1".to_string(),
            code: vec![],
            input: "".to_string(),
        };

        let response = run_code(run_request, &mock_service).await;
        assert_eq!(response.status, 1);
        assert_eq!(response.stdout, "");
        assert_eq!(response.stderr, "Nothing was run");
    }

    #[tokio::test]
    async fn test_run_with_steps() {
        let mut mock_service = MockLambdoApiServiceTrait::new();
        mock_service.expect_run_code().once().returning(|_| {
            Ok(ExecuteResponse {
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
            })
        });

        let run_request = RunRequest {
            language: "Node".to_string(),
            version: "1".to_string(),
            code: vec![FileModel {
                filename: "test.js".to_string(),
                content: "console.log('Hello World')".to_string(),
            }],
            input: "test.js".to_string(),
        };

        let response = run_code(run_request, &mock_service).await;
        assert_eq!(response.status, 0);
        assert_eq!(response.stdout, "HelloWorld");
        assert_eq!(response.stderr, "");
    }
}
