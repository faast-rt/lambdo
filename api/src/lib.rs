pub mod config;
pub mod controller;
pub mod model;
pub mod net;
pub mod service;
pub mod vmm;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LambdoError {
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    #[error("unknown lambdo error")]
    Unknown,
}
