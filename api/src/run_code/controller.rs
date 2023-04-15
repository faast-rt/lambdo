use actix_web::{post, web, Responder};
use log::{info, error};

use crate::run_code::model::{RunRequest, RunResponse};
use std::error::Error;

use crate::run_code::service::run_vmm;

#[post("/run")]
async fn run(_run_body: web::Json<RunRequest>) -> Result<impl Responder, Box<dyn Error>> {
    info!("Running code");
    let stdout = run_vmm();
    info!("Execution finished");

    let response = match stdout {
        Ok(stdout) => {
            RunResponse {
                status: 0,
                stdout: stdout,
                stderr: "".to_string(),
            }
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
