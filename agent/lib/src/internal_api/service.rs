use std::{process::Command, fs::File, path::{PathBuf, Path}};
use anyhow::{anyhow, Result};
use log::{error, info};
use crate::{external_api::model::{RequestMessage, ResponseMessage, ResponseData, ResponseStep}, internal_api::model::FileModel};
use std::io::Write;
use super::model::{CodeReturn, InternalError};

const WORKSPACE_PATH: &str = "/tmp";

pub struct InternalApi {
    pub request_message: RequestMessage,
}

impl InternalApi {
    pub fn new(request_message: RequestMessage) -> Self {
        Self { request_message }
    }

    pub fn create_workspace(&mut self) -> Result<()> {
        info!("Creating workspace for code execution");

        // Create a vector of FileModel and a root path
        let mut file_models: Vec<FileModel> = Vec::new();
        let root_path = PathBuf::from(WORKSPACE_PATH);

        self.request_message.data.files.iter().for_each(|file| {
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

    pub fn run(&mut self) -> Result<ResponseMessage, InternalError> {
        info!("Running code");
        
        // Running the latest command in vector for now
        
        let child_process = Command::new("/bin/sh")
            .args(["-c", 
                self.request_message.data.steps.last().ok_or(InternalError::CmdSpawn)?.command.as_str()
            ])
            .current_dir(WORKSPACE_PATH)
            .output()
            .map_err(|_|InternalError::CmdSpawn)?;

        info!("Code execution finished, gathering outputs and exit code");

        let exit_code = child_process.status.code().ok_or(
            InternalError::InvalidExitCode
        )?;
        let stdout = String::from_utf8(child_process.stdout).map_err(
            |_| InternalError::StdoutRead
        )?;
        let stderr = String::from_utf8(child_process.stderr).map_err(
            |_| InternalError::StderrRead
        )?;
        let step = ResponseStep::new(self.request_message.data.steps.last().ok_or(InternalError::CmdSpawn)?.command.clone(), exit_code, stdout.clone(), stderr.clone(), false);
        let steps = vec![step];
        let data: ResponseData = ResponseData::new(stdout.clone(), steps);
        // let response_message = ResponseMessage::new(
        // Ok(CodeReturn::new(stdout, stderr, exit_code))
    }

}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;
    use crate::external_api::model::{FileModel, CodeEntry};
    use super::*;

    fn random_usize(max: usize) -> usize {
        let mut f = File::open("/dev/urandom").unwrap();
        let mut buf = [0u8; 1];
        f.read_exact(&mut buf).unwrap();
        let value = buf[0] as usize;

        if value < max {
            max
        } else {
            value % max
        }
    }

    fn native_rand_string(len: usize) -> String {
        let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
        let mut string = String::new();

        for _ in  0..len {
            string.push(chars.chars().nth(random_usize(chars.len() - 1)).unwrap());
        }

        string
    }

    #[test]
    fn workload_runs_correctly() {
        let entry = CodeEntry {
            files: vec![],
            script: vec![String::from("echo 'This is stdout' && echo 'This is stderr' >&2")],
        };


        let mut api = InternalApi::new(entry); // Empty code entry to avoid borrowing issues 
            // since the same object is used in the `run` method

        let res = api.run().unwrap();

        assert_eq!(res.exit_code, 0);
        assert_eq!(res.stderr, "This is stderr\n");
        assert_eq!(res.stdout, "This is stdout\n");
    }

    #[test]
    fn workspace_created_sucessfully() {
        let mut base_dir = PathBuf::from(WORKSPACE_PATH);
        base_dir.push(native_rand_string(20));
        base_dir.push("main.sh");
        let path = base_dir.into_os_string().into_string().unwrap();


        let entry = CodeEntry { 
            files: vec![
                FileModel {
                    filename: path.clone(),
                    content: "#!/bin/sh\necho -n 'Some outpout'".to_string()
                }
            ],
            script: vec![path.clone()],
        };

        InternalApi::new(entry).create_workspace().unwrap();

        assert!(Path::new(&path).exists());
    }
}