use super::model::{CodeReturn, InternalError};
use crate::{external_api::model::CodeEntry, internal_api::model::FileModel};
use anyhow::{anyhow, Result};
use log::{error, info};
use std::io::Write;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};
use unshare::Command;

const WORKSPACE_PATH: &str = "/tmp";

pub struct InternalApi {
    pub code_entry: CodeEntry,
}

impl InternalApi {
    pub fn new(code_entry: CodeEntry) -> Self {
        Self { code_entry }
    }

    pub fn create_workspace(&mut self) -> Result<()> {
        info!("Creating workspace for code execution");

        // Create a vector of FileModel and a root path
        let mut file_models: Vec<FileModel> = Vec::new();
        let root_path = PathBuf::from(WORKSPACE_PATH);

        self.code_entry.files.iter().for_each(|file| {
            let mut file_path = PathBuf::from(&file.filename);
            file_path.pop();

            // Add `/tmp` before each path
            file_path = root_path.join(file_path);

            // Take the file name and add it to the vector of files
            let file_name = Path::file_name(Path::new(&file.filename));

            // Extract the file name from the path and create a FileModel
            if let Some(file_name_str) = file_name {
                let fns = file_name_str.to_os_string();
                let file_name_string_option = fns.to_str();

                if let Some(file_name_string) = file_name_string_option {
                    let file_model = FileModel::new(
                        file_path,
                        file_name_string.to_string(),
                        file.content.clone(),
                    );
                    file_models.push(file_model);
                } else {
                    error!("Failed to convert file name to string");
                }
            } else {
                error!("Failed to extract file name from path");
            }
        });

        info!("Final file models: {:?}", file_models);

        // For each file model, create the directory and the file
        file_models.iter().for_each(|file_model| {
            let file_path = file_model.path.clone();
            let file_name = file_model.file_name.clone();

            // Create the directory
            let op_dir = std::fs::create_dir_all(&file_path)
                .map_err(|e| anyhow!("Failed to create directory: {}", e));
            if op_dir.is_err() {
                error!("Failed to create directory: {:?}", op_dir.err());
            } else {
                info!("Directory created: {:?}", file_path);
            }

            // Create the file
            let file_path = file_path.join(file_name);
            let op_file =
                File::create(file_path).map_err(|e| anyhow!("Failed to create file: {}", e));

            if let Err(e) = op_file {
                error!("Failed to create file: {:?}", e);
            } else {
                let mut file = op_file.unwrap();
                info!("File created: {:?}", file);

                // Write the content inside the file
                let res = write!(file, "{}", file_model.content);

                if let Err(err) = res {
                    error!("Failed to write to file: {:?}", err);
                } else {
                    info!("File written: {:?}", file);
                }
            }
        });

        Ok(())
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
            stdout_reader
                .read_to_string(&mut stdout_output)
                .map_err(|_| InternalError::StdoutRead)?;
            // println!("{}", stdout_output);
            let result = CodeReturn::new(stdout_output, "stderr".to_string(), 0);

            Ok(result)
        } else {
            println!("No exit code");
            Err(InternalError::InvalidExitCode)
        }
    }
}
