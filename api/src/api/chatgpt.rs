use std::error::Error;

use actix_files::NamedFile;
use actix_web::{get, Responder};
use log::debug;

#[get("/.well-known/ai-plugin.json")]
pub async fn ai_plugin() -> Result<impl Responder, Box<dyn Error>> {
    debug!("Received ai-plugin request from http",);

    Ok(NamedFile::open_async("./api/static/ai-plugin.json").await?)
}
