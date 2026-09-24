use super::*;
use yoctui_bitbake::{
    BackendError, DependencyGraphResponse, RecipeDependencies, SignatureComparisonResponse,
    SignatureDumpResponse,
};
use yoctui_model::{Layer, Recipe, RecipeMetadata, Workspace};

struct PendingMetadataBackend;

#[async_trait::async_trait]
impl BitBakeBackend for PendingMetadataBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn list_recipes(&mut self, _filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn get_variable(
        &mut self,
        _name: String,
        _recipe: Option<String>,
    ) -> Result<VariableValue, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn get_dependencies(
        &mut self,
        _recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn get_dependency_graph(
        &mut self,
        _recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn get_signature_dump(
        &mut self,
        _target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn compare_signatures(
        &mut self,
        _request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn get_recipe_sources(&mut self, _recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn get_recipe_metadata(
        &mut self,
        _recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
        std::future::pending().await
    }

    async fn get_layer_relationships(
        &mut self,
    ) -> Result<Vec<yoctui_bitbake::LayerRelationship>, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn start_build(&mut self, _request: BuildRequest) -> Result<(), BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        Ok(())
    }

    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn shutdown(&mut self) -> Result<(), BackendError> {
        Ok(())
    }
}

#[tokio::test]
async fn platform_inspection_worker_is_cancellable_while_metadata_is_pending() {
    let worker = tokio::spawn(async {
        let mut backend = PendingMetadataBackend;
        inspect_kernel_workbench(None, &mut backend).await
    });
    tokio::task::yield_now().await;
    assert!(!worker.is_finished());

    worker.abort();
    let result = tokio::time::timeout(Duration::from_millis(100), worker)
        .await
        .expect("platform inspection worker did not stop after cancellation");
    assert!(
        result
            .expect_err("cancelled worker unexpectedly completed")
            .is_cancelled()
    );
}
