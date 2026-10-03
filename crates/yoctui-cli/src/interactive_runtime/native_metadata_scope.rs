//! Private native read-only connections must not reserve BitBake while idle.
use super::*;
use std::{future::Future, pin::Pin};
use yoctui_bitbake::{
    BackendError, DependencyGraphResponse, LayerRelationship, RecipeDependencies,
    SignatureComparisonResponse, SignatureDumpResponse,
};
use yoctui_model::{Layer, Recipe, RecipeMetadata, Workspace};

type BackendFuture =
    Pin<Box<dyn Future<Output = Result<Box<dyn BitBakeBackend>, BackendError>> + Send>>;
type BackendFactory = Box<dyn Fn() -> BackendFuture + Send + Sync>;

pub(super) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) struct NativeMetadataScope {
    factory: BackendFactory,
    shutdown_timeout: Duration,
}

impl NativeMetadataScope {
    pub(super) fn new(
        build_dir: PathBuf,
        source_dir: Option<PathBuf>,
        cancellation_timeout: Duration,
    ) -> Self {
        Self {
            factory: Box::new(move || {
                let build_dir = build_dir.clone();
                let source_dir = source_dir.clone();
                Box::pin(async move {
                    let environment = metadata_backend::initialized_metadata_environment(
                        &build_dir,
                        source_dir.as_deref(),
                    )
                    .await
                    .map_err(|error| BackendError::Bridge(format!("{error:#}")))?;
                    select_backend_with_environment(
                        Backend::Bridge,
                        build_dir,
                        Some(cancellation_timeout),
                        Some(environment),
                    )
                    .await
                    .map_err(|error| BackendError::Bridge(format!("{error:#}")))
                })
            }),
            shutdown_timeout: SHUTDOWN_TIMEOUT,
        }
    }
}

pub(super) async fn finish_query<T>(
    mut backend: Box<dyn BitBakeBackend>,
    result: Result<T, BackendError>,
    timeout: Duration,
) -> Result<T, BackendError> {
    let cleanup = match tokio::time::timeout(timeout, backend.shutdown()).await {
        Ok(result) => result,
        Err(_) => Err(BackendError::Bridge(
            "native metadata connection shutdown timed out".into(),
        )),
    };
    if let Err(cleanup) = cleanup {
        let detail = result
            .err()
            .map(|error| format!("; inspection also failed: {error}"))
            .unwrap_or_default();
        return Err(BackendError::Bridge(format!(
            "cannot release native metadata connection: {cleanup}{detail}"
        )));
    }
    result
}

#[async_trait::async_trait]
impl BitBakeBackend for NativeMetadataScope {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.inspect_workspace().await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.list_recipes(filter).await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.list_layers().await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn get_variable(
        &mut self,
        name: String,
        recipe: Option<String>,
    ) -> Result<VariableValue, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.get_variable(name, recipe).await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.get_dependencies(recipe).await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.get_dependency_graph(recipe).await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn get_signature_dump(
        &mut self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.get_signature_dump(target).await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn compare_signatures(
        &mut self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.compare_signatures(request).await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn get_recipe_sources(&mut self, recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.get_recipe_sources(recipe).await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn get_recipe_metadata(
        &mut self,
        recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.get_recipe_metadata(recipe).await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        let mut backend = (self.factory)().await?;
        let result = backend.get_layer_relationships().await;
        finish_query(backend, result, self.shutdown_timeout).await
    }

    async fn start_build(&mut self, _request: BuildRequest) -> Result<(), BackendError> {
        Err(BackendError::Bridge(
            "Native builds belong to the attached daemon, not a metadata inspection.".into(),
        ))
    }

    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        Err(BackendError::NotRunning)
    }

    async fn shutdown(&mut self) -> Result<(), BackendError> {
        // No persistent process: each query already released its owned child.
        Ok(())
    }
}

pub(super) fn hold_pending_inspection_launch(
    app: &mut App,
    native: bool,
    inspection_pending: bool,
    input: Input,
) -> bool {
    if !native || !inspection_pending || input != Input::Enter {
        return false;
    }
    let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog() else {
        return false;
    };
    use yoctui_model::TerminalCreationKind;
    let request = &dialog.request;
    let bitbake_shell = matches!(
        request.kind,
        TerminalCreationKind::BuildShell
            | TerminalCreationKind::DevtoolShell
            | TerminalCreationKind::Devshell
            | TerminalCreationKind::Menuconfig
    );
    let tool = |value: &Path| {
        matches!(
            value.file_name().and_then(|v| v.to_str()),
            Some("bitbake" | "devtool")
        )
    };
    let bitbake_utility = request.kind == TerminalCreationKind::Utility
        && (tool(&request.program)
            || request
                .arguments
                .first()
                .is_some_and(|value| tool(Path::new(value))));
    if !bitbake_shell && !bitbake_utility {
        return false;
    }
    app.notification = Some(
        "Wait for metadata inspection to release BitBake, then confirm again; Esc cancels.".into(),
    );
    true
}

#[cfg(test)]
#[path = "../tests/inspection_lease.rs"]
mod tests;
