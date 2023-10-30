use super::model::CodeReturn;
use crate::api::grpc_definitions::{ExecuteRequest, ExecuteResponse, ExecuteResponseStep};
use crate::runner_engine::model::FileModel;
use anyhow::{anyhow, Ok, Result};
use log::{error, info};
use std::io::Write;
use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Command,
};

/// The path where the workspace will be created
const WORKSPACE_PATH: &str = "/tmp";

/// The RunnerEngine API
pub struct RunnerEngine {
    pub request_message: ExecuteRequest,
}

impl RunnerEngine {
    /// Create a new instance of RunnerEngine
    ///
    /// # Arguments
    ///
    /// * `request_message` - The request message
    ///
    /// # Returns
    ///
    /// * `Self` - The new instance of RunnerEngine
    pub fn new(request_message: ExecuteRequest) -> Self {
        Self { request_message }
    }

    /// Create the workspace for the code execution
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Nothing or an error
    pub fn create_workspace(&mut self) -> Result<()> {
        info!("Creating workspace for code execution");

        // Create a vector of FileModel and a root path
        let mut file_models: Vec<FileModel> = Vec::new();
        let root_path = PathBuf::from(WORKSPACE_PATH);

        self.request_message.files.iter().for_each(|file| {
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

    /// Run all the steps of the request message
    ///
    /// # Returns
    ///
    /// * `Result<ResponseMessage>` - The response message or an error
    pub fn run(&mut self) -> Result<ExecuteResponse> {
        info!("Running all steps");
        let mut steps: Vec<ExecuteResponseStep> = Vec::new();

        // For each commands in the request, run it
        let steps_to_process = self.request_message.steps.clone();

        for step in steps_to_process {
            let command = step.command.as_str();
            let code_return = self.run_one(command)?;

            // Hide Stdout if enable_output is false
            let stdout = if step.enable_output {
                code_return.stdout
            } else {
                String::new()
            };
            let response_step = ExecuteResponseStep {
                command: command.to_string(),
                exit_code: code_return.exit_code,
                stdout,
                stderr: code_return.stderr,
            };

            steps.push(response_step);
        }

        let data = ExecuteResponse {
            id: self.request_message.id.clone(),
            steps,
        };

        Ok(data)
    }

    /// Run a command
    ///
    /// # Arguments
    ///
    /// * `command` - The command to run
    ///
    /// # Returns
    ///
    /// * `Result<CodeReturn>` - The code return or an error
    pub fn run_one(&mut self, command: &str) -> Result<CodeReturn> {
        info!("Running command : {}", command);

        let child_process = Command::new("/bin/sh")
            .args(["-c", command])
            .current_dir(WORKSPACE_PATH)
            .output()
            .map_err(|e| anyhow!("Failed to spawn command : {}", e))?;

        let exit_code = child_process
            .status
            .code()
            .ok_or_else(|| anyhow!("Failed to retrieve exit_code"))?;
        let stdout = String::from_utf8(child_process.stdout)
            .map_err(|e| anyhow!("Failed to retrieve stdout stream : {}", e))?;
        let stderr = String::from_utf8(child_process.stderr)
            .map_err(|e| anyhow!("Failed to retrieve stderr stream : {}", e))?;

        let code_return = CodeReturn::new(stdout, stderr, exit_code);

        info!("Code execution finished: {:?}", code_return);
        Ok(code_return)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::grpc_definitions::{ExecuteRequestStep, FileModel};

    use super::*;
    use rand::random;
    use std::fs::File;
    use std::io::Read;

    /// Generate a random string
    ///
    /// # Arguments
    ///
    /// * `len` - The length of the string
    ///
    /// # Returns
    ///
    /// * `String` - The random string
    fn native_rand_string(len: usize) -> String {
        let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
        let mut string = String::new();

        for _ in 0..len {
            string.push(
                chars
                    .chars()
                    .nth(random::<usize>() % (chars.len() - 1))
                    .unwrap(),
            );
        }

        string
    }

    #[test]
    fn run_one_works_with_ouputs_and_code() {
        let res = RunnerEngine::new(ExecuteRequest { 
            id: "".to_string(),
            files: vec![], 
            steps: vec![] 
        }).run_one("echo -n 'This is stdout' && echo -n 'This is stderr' >&2 && exit 1");   

        assert!(res.is_ok());

        let code_return = res.unwrap();

        assert_eq!(code_return.stdout, "This is stdout");
        assert_eq!(code_return.stderr, "This is stderr");
        assert_eq!(code_return.exit_code, 1);
    }

    /// Test the creation of a file
    #[test]
    fn workload_runs_correctly() {
        let files: Vec<FileModel> = Vec::new();
        let steps: Vec<ExecuteRequestStep> = vec![
            ExecuteRequestStep {
                command: "echo 'This is stdout' && echo 'This is stderr' >&2".to_string(),
                enable_output: true,
            },
            ExecuteRequestStep {
                command: "echo 'This is stdout' && echo 'This is stderr' >&2 && exit 1".to_string(),
                enable_output: false,
            },
        ];

        let request_data = ExecuteRequest {
            id: "4bf68974-c315-4c41-aee2-3dc2920e76e9".to_string(),
            files,
            steps,
        };

        let mut api = RunnerEngine::new(request_data);

        let res = api.run().unwrap();

        assert_eq!(res.steps[0].exit_code, 0);
        assert_eq!(res.steps[0].stderr, "This is stderr\n");
        assert_eq!(res.steps[0].stdout, "This is stdout\n");

        println!("{:?}", res.steps[1]);
        assert_eq!(res.steps[1].exit_code, 1);
        assert_eq!(res.steps[1].stderr, "This is stderr\n");
        assert!(res.steps[1].stdout.is_empty());
        
        assert_eq!(res.id, "4bf68974-c315-4c41-aee2-3dc2920e76e9");
    }

    /// Test the execution of a command with a workspace
    #[test]
    fn workspace_created_sucessfully() {
        let mut base_dir = PathBuf::from(WORKSPACE_PATH);
        base_dir.push(native_rand_string(20));
        base_dir.push("main.sh");
        let path = base_dir.into_os_string().into_string().unwrap();

        let files: Vec<FileModel> = vec![FileModel {
            filename: path.clone(),
            content: "Hello World!".to_string(),
        }];
        let steps: Vec<ExecuteRequestStep> = Vec::new();
        let request_data = ExecuteRequest {
            id: "4bf68974-c315-4c41-aee2-3dc2920e76e9".to_string(),
            files,
            steps,
        };

        RunnerEngine::new(request_data).create_workspace().unwrap();

        assert!(Path::new(&path).exists());

        //Check that the file contains the specified content
        let mut file = File::open(&path).unwrap();
        let mut buffer = [0; 12];
        file.read_exact(&mut buffer[..]).unwrap();

        // Convert buffer to string
        let content = String::from_utf8(buffer.to_vec()).unwrap();
        assert!(file.metadata().unwrap().is_file());
        assert_eq!(content, "Hello World!");
    }
}
