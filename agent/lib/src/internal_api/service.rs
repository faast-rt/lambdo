use std::io::{BufReader,Read};

use super::model::{CodeReturn, InternalError};
use unshare::Command;

pub struct InternalApi {
  pub entrypoint: String,
  pub code: String,
}

impl InternalApi {
  pub fn new(entrypoint: String, code: String) -> Self {
    Self {
      entrypoint,
      code,
    }
  }

  pub fn write_log(&self) -> String {
    "Hello".to_string()
  }

  pub fn run(&mut self) -> Result<CodeReturn, InternalError> {

    let code = Command::new("/bin/sh")
    .args(&["-c", "echo", "'Hello world'"])
    .spawn()
    .map_err(InternalError::CmdSpawn)?
    .stdout;



    // .wait().map_err(InternalError::ChildWait)?.fmt(f)
    // .wait()
    // .map_err(InternalError::ChildWait)?
    // .code();

    if let Some(code) = code {
      // if code != 0 {
      //     return Err(InternalError::ChildExitError(code));
      // }
      let mut stdout_reader = BufReader::new(code);
      let mut stdout_output = String::new();
      println!("Internal API: Reading stdout");
      stdout_reader.read_to_string(&mut stdout_output).map_err(|_| InternalError::StdoutRead)?;
      // println!("{}", stdout_output);
      let result = CodeReturn::new(stdout_output, "stderr".to_string(), 0);
    
      Ok(result)

    } else {
      println!("No exit code");
      Err(InternalError::InvalidExitCode)
    }

    
  }

}