use actix_web::{App,  HttpServer};
mod run_code;
use run_code::controller::run;
use log::info;
use std::env;




#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let port = env::var("SERVER_PORT").unwrap_or("8080".to_string());
    let address = env::var("SERVER_ADDRESS").unwrap_or("127.0.0.1".to_string());

    info!("Starting server");
    HttpServer::new(|| {
        App::new()
            .service(run)
    })
    .bind((address, port.parse().unwrap()))?
    .run()
    .await
}