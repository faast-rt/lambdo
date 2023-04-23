use actix_web::{post, web, Responder};
use log::{debug, error, info, trace};
use tokio::sync::Mutex;

use crate::{
    grpc_definitions::ExecuteResponse,
    model::{RunRequest, RunResponse},
    LambdoState,
};
use std::{error::Error, sync::Arc};

use crate::service::run_code;

#[post("/run")]
async fn run(
    run_body: web::Json<RunRequest>,
    state: web::Data<Arc<Mutex<LambdoState>>>,
) -> Result<impl Responder, Box<dyn Error>> {
    debug!(
        "Received code execution request from http (language: {}, version: {})",
        run_body.language, run_body.version
    );
    trace!("Request body: {:?}", run_body);

    let response = run_code(state, run_body).await;

    let response = match response {
        Ok(response) => {
            info!("Execution ended for {:?}", response.id);
            trace!("Response: {:?}", response);
            parse_response(response)
        }
        // for the moment just signal an internal server error
        Err(e) => {
            error!("Error while executing code: {:?}", e);
            RunResponse {
                status: 1,
                stdout: "".to_string(),
                stderr: "Internal server error".to_string(),
            }
        }
    };

    Ok(web::Json(response))
}

fn parse_response(response: ExecuteResponse) -> RunResponse {
    let mut stdout = String::new();
    let mut stderr = String::new();
    for step in response.steps.as_slice() {
        if step.stdout.is_empty() {
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
