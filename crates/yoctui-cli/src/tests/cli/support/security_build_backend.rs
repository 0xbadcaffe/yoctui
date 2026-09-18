use super::*;

pub(crate) struct SecurityBuildBackend {
    pub(crate) started: std::sync::Arc<std::sync::Mutex<Vec<BuildRequest>>>,
    pub(crate) fail_start: bool,
}

#[async_trait::async_trait]
impl BitBakeBackend for SecurityBuildBackend {
    async fn inspect_workspace(
        &mut self,
    ) -> std::result::Result<yoctui_model::Workspace, yoctui_bitbake::BackendError> {
        Ok(yoctui_model::Workspace::default())
    }

    async fn list_recipes(
        &mut self,
        _filter: Option<String>,
    ) -> std::result::Result<Vec<yoctui_model::Recipe>, yoctui_bitbake::BackendError> {
        Ok(Vec::new())
    }

    async fn list_layers(
        &mut self,
    ) -> std::result::Result<Vec<yoctui_model::Layer>, yoctui_bitbake::BackendError> {
        Ok(Vec::new())
    }

    async fn get_variable(
        &mut self,
        _name: String,
        _recipe: Option<String>,
    ) -> std::result::Result<VariableValue, yoctui_bitbake::BackendError> {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn get_dependencies(
        &mut self,
        _recipe: String,
    ) -> std::result::Result<yoctui_bitbake::RecipeDependencies, yoctui_bitbake::BackendError> {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn get_dependency_graph(
        &mut self,
        _recipe: String,
    ) -> std::result::Result<yoctui_bitbake::DependencyGraphResponse, yoctui_bitbake::BackendError>
    {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn get_signature_dump(
        &mut self,
        _target: SignatureTarget,
    ) -> std::result::Result<yoctui_bitbake::SignatureDumpResponse, yoctui_bitbake::BackendError>
    {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn compare_signatures(
        &mut self,
        _request: SignatureComparisonRequest,
    ) -> std::result::Result<
        yoctui_bitbake::SignatureComparisonResponse,
        yoctui_bitbake::BackendError,
    > {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn get_recipe_sources(
        &mut self,
        _recipe: String,
    ) -> std::result::Result<Vec<PathBuf>, yoctui_bitbake::BackendError> {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn get_recipe_metadata(
        &mut self,
        _recipe: String,
    ) -> std::result::Result<yoctui_model::RecipeMetadata, yoctui_bitbake::BackendError> {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn get_layer_relationships(
        &mut self,
    ) -> std::result::Result<Vec<yoctui_bitbake::LayerRelationship>, yoctui_bitbake::BackendError>
    {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn start_build(
        &mut self,
        request: BuildRequest,
    ) -> std::result::Result<(), yoctui_bitbake::BackendError> {
        if self.fail_start {
            Err(yoctui_bitbake::BackendError::Bridge(
                "synthetic Security start failure".into(),
            ))
        } else {
            self.started.lock().unwrap().push(request);
            Ok(())
        }
    }

    async fn cancel_build(&mut self) -> std::result::Result<(), yoctui_bitbake::BackendError> {
        Ok(())
    }

    async fn next_event(
        &mut self,
    ) -> std::result::Result<BackendEvent, yoctui_bitbake::BackendError> {
        Err(yoctui_bitbake::BackendError::NotRunning)
    }

    async fn shutdown(&mut self) -> std::result::Result<(), yoctui_bitbake::BackendError> {
        Ok(())
    }
}
