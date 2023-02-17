use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct CodeSubmission {
  pub entrypoint: String,
  pub code: String,
}

impl CodeSubmission {
  pub fn new(entrypoint: String, code: String) -> Self {
    Self {
      entrypoint,
      code,
    }
  }
}