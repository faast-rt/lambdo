use actix_web::{post, web, Responder};
use log::{error, info};
use shared::ResponseMessage;

use crate::{
    config::LambdoConfig,
    model::{RunRequest, RunResponse},
};
use std::error::Error;

use crate::service::run_code;

#[post("/run")]
async fn run(
    run_body: web::Json<RunRequest>,
    config: web::Data<LambdoConfig>,
) -> Result<impl Responder, Box<dyn Error>> {
    info!("Running code");
    let response = run_code(config, run_body);
    info!("Execution finished");

    let response = match response {
        Ok(response) => {
            info!("Response: {:?}", response);
            parse_response(response)
        }
        // for the moment just signal an internal server error
        Err(e) => {
            error!("Error: {:?}", e);
            RunResponse {
                status: 1,
                stdout: "".to_string(),
                stderr: "Internal server error".to_string(),
            }
        }
    };
    Ok(web::Json(response))
}

fn parse_response(response: ResponseMessage) -> RunResponse {
    let mut stdout = String::new();
    let mut stderr = String::new();
    for step in response.data.steps.as_slice() {
        if step.stdout.is_some() {
            stdout.push_str(step.stdout.as_ref().unwrap().as_str());
        }
        stderr.push_str(step.stderr.as_str());
    }

    RunResponse {
        status: response.data.steps[response.data.steps.len() - 1]
            .exit_code
            .try_into()
            .unwrap(),
        stdout,
        stderr,
    }
}
