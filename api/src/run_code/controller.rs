use actix_web::{post, Responder, web};
use log::info;

use crate::run_code::model::{RunResponse, RunRequest};
use std::error::Error;

#[post("/run")]
async fn run(run_body: web::Json<RunRequest>) -> Result<impl Responder, Box<dyn Error>> {
    info!("Running code");
    //TODO: run code
    info!("Execution finished");
    let response = RunResponse {
        status: 1,
        stdout: "Not implemented".to_string(),
        stderr: "Not implemented".to_string()
    };
    Ok(web::Json(response))
}