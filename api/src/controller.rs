use actix_web::{post, web, Responder};
use log::{info, error};
use shared::config::LambdoConfig;

use crate::model::{RunRequest, RunResponse};
use std::error::Error;

use crate::service::run_vmm;

#[post("/run")]
async fn run(_run_body: web::Json<RunRequest>, config: web::Data<LambdoConfig>) -> Result<impl Responder, Box<dyn Error>> {
    info!("Running code");
    let stdout = run_vmm(config.vmm.kernel.clone());
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
