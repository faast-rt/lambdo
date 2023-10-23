use crate::{
    config::{LambdoConfig, LambdoLanguageConfig},
    vm_manager::grpc_definitions::{
        ExecuteRequest, ExecuteRequestStep, ExecuteResponse, FileModel,
    },
    vm_manager::{state::LambdoStateRef, Error, VMManager},
};
use log::{debug, trace};
use uuid::Uuid;

use crate::model::RunRequest;

pub struct LambdoApiService {
    pub config: LambdoConfig,
    pub vm_manager: VMManager,
}

impl LambdoApiService {
    pub async fn new(config: LambdoConfig) -> Result<Self, Error> {
        let state = crate::vm_manager::state::LambdoState::new(config.clone());
        let vm_manager =
            VMManager::new(std::sync::Arc::new(tokio::sync::Mutex::new(state))).await?;
        Ok(LambdoApiService { config, vm_manager })
    }

    pub async fn new_with_state(state: LambdoStateRef) -> Result<Self, Error> {
        let config = state.lock().await.config.clone();
        let vm_manager = VMManager::new(state).await?;
        Ok(LambdoApiService { config, vm_manager })
    }

    pub async fn run_code(&self, request: RunRequest) -> Result<ExecuteResponse, Error> {
        let entrypoint = request.code[0].filename.clone();

        let language_settings = self.find_language(&request.language).unwrap();
        let steps = self.generate_steps(&language_settings, &entrypoint);
        let file = FileModel {
            filename: entrypoint.to_string(),
            content: request.code[0].content.clone(),
        };
        let input_filename = "input.input";

        let input = FileModel {
            filename: input_filename.to_string(),
            content: request.input.clone(),
        };

        let request_data = ExecuteRequest {
            id: Uuid::new_v4().to_string(),
            steps,
            files: vec![file, input],
        };
        trace!("Request message to VMM: {:?}", request_data);

        let response = self
            .vm_manager
            .run_code(request_data, language_settings.into())
            .await;
        debug!("Response from VMM: {:?}", response);

        response
    }

    fn find_language(
        &self,
        language: &String,
    ) -> Result<LambdoLanguageConfig, Box<dyn std::error::Error>> {
        let language_list = &self.config.languages;
        for lang in language_list {
            if &*lang.name == language {
                return Ok(lang.clone());
            }
        }
        Err("Language not found".into())
    }

    fn generate_steps(
        &self,
        language_settings: &LambdoLanguageConfig,
        entrypoint: &str,
    ) -> Vec<ExecuteRequestStep> {
        let mut steps: Vec<ExecuteRequestStep> = Vec::new();
        for step in &language_settings.steps {
            let command = step.command.replace("{{filename}}", entrypoint);

            steps.push(ExecuteRequestStep {
                command,
                enable_output: step.output.enabled,
            });
        }
        steps
    }
}
