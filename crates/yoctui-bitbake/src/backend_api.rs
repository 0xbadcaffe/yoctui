//! Backend api.
use super::*;

#[derive(Debug, Clone)]
pub enum BackendEvent {
    SstateSummary(yoctui_model::SstateSummary),
    Workspace(Workspace),
    Recipes(Vec<Recipe>),
    Layers(Vec<Layer>),
    Variable {
        name: String,
        recipe: Option<String>,
        value: Option<String>,
        provenance: Option<String>,
        unexpanded_value: Option<String>,
        operations: Vec<VariableOperation>,
        active_overrides: Vec<String>,
    },
    Dependencies {
        recipe: String,
        build: Vec<String>,
        runtime: Vec<String>,
    },
    DependencyGraph {
        graph: DependencyGraph,
        limitations: Vec<String>,
    },
    DependencyGraphFailed {
        root: DependencyNodeId,
        message: String,
    },
    SignatureDump {
        target: SignatureTarget,
        records: Vec<SignatureRecord>,
        limitations: Vec<String>,
    },
    SignatureDumpFailed {
        target: SignatureTarget,
        message: String,
    },
    SignatureComparison {
        request: SignatureComparisonRequest,
        differences: Vec<SignatureDifference>,
        limitations: Vec<String>,
    },
    SignatureComparisonFailed {
        request: SignatureComparisonRequest,
        message: String,
    },
    PackageInventory {
        request: PackageInventoryRequest,
        packages: Vec<PackageSummary>,
        limitations: Vec<String>,
    },
    PackageInventoryFailed {
        request: PackageInventoryRequest,
        message: String,
    },
    PackageDetail {
        request: PackageDetailRequest,
        detail: PackageDetail,
        limitations: Vec<String>,
    },
    PackageDetailFailed {
        request: PackageDetailRequest,
        message: String,
    },
    ImageArtifacts {
        request: ImageArtifactRequest,
        inventory: ImageArtifactInventory,
        limitations: Vec<String>,
    },
    ImageArtifactsFailed {
        request: ImageArtifactRequest,
        message: String,
    },
    RootfsComposition {
        request: RootfsCompositionRequest,
        composition: RootfsComposition,
        limitations: Vec<String>,
    },
    RootfsCompositionUnavailable {
        request: RootfsCompositionRequest,
        reason: String,
    },
    RootfsCompositionFailed {
        request: RootfsCompositionRequest,
        message: String,
    },
    RecipeSources {
        recipe: String,
        paths: Vec<PathBuf>,
    },
    RecipeMetadata(RecipeMetadata),
    LayerRelationships(Vec<LayerRelationship>),
    BuildStarted,
    TaskStats(TaskStats),
    ParseProgress {
        current: Option<u64>,
        total: Option<u64>,
    },
    Log(LogEntry),
    TaskQueued {
        recipe: String,
        task: String,
        worker: Option<String>,
        stats: Option<TaskStats>,
    },
    TaskStarted {
        recipe: String,
        task: String,
        pid: Option<u32>,
        worker: Option<String>,
        log_path: Option<PathBuf>,
        stats: Option<TaskStats>,
    },
    TaskProgress {
        recipe: String,
        task: String,
        progress: Option<u8>,
    },
    TaskCompleted {
        recipe: String,
        task: String,
        success: bool,
    },
    BuildCompleted {
        success: bool,
        exit_code: Option<i32>,
    },
    CommandFailed {
        code: String,
        message: String,
    },
    Ignored,
    Disconnected,
}

impl From<SignatureDumpResponse> for BackendEvent {
    fn from(response: SignatureDumpResponse) -> Self {
        Self::SignatureDump {
            target: response.target,
            records: response.records,
            limitations: response.limitations,
        }
    }
}

impl From<SignatureComparisonResponse> for BackendEvent {
    fn from(response: SignatureComparisonResponse) -> Self {
        Self::SignatureComparison {
            request: response.request,
            differences: response.differences,
            limitations: response.limitations,
        }
    }
}

impl From<PackageInventoryResponse> for BackendEvent {
    fn from(response: PackageInventoryResponse) -> Self {
        Self::PackageInventory {
            request: response.request,
            packages: response.packages,
            limitations: response.limitations,
        }
    }
}

impl From<PackageDetailResponse> for BackendEvent {
    fn from(response: PackageDetailResponse) -> Self {
        Self::PackageDetail {
            request: response.request,
            detail: response.detail,
            limitations: response.limitations,
        }
    }
}

impl From<ImageArtifactResponse> for BackendEvent {
    fn from(response: ImageArtifactResponse) -> Self {
        Self::ImageArtifacts {
            request: response.request,
            inventory: response.inventory,
            limitations: response.limitations,
        }
    }
}

impl From<RootfsCompositionResponse> for BackendEvent {
    fn from(response: RootfsCompositionResponse) -> Self {
        Self::RootfsComposition {
            request: response.request,
            composition: response.composition,
            limitations: response.limitations,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableValue {
    pub recipe: Option<String>,
    pub value: Option<String>,
    pub provenance: Option<String>,
    pub unexpanded_value: Option<String>,
    pub operations: Vec<VariableOperation>,
    pub active_overrides: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecipeDependencies {
    pub build: Vec<String>,
    pub runtime: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraphResponse {
    pub graph: DependencyGraph,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerRelationship {
    pub name: String,
    pub priority: Option<i32>,
    pub compatible: Vec<String>,
    pub depends: Vec<String>,
    pub overlays: Vec<String>,
    pub appends: Vec<String>,
}

#[async_trait]
pub trait BitBakeBackend: Send {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError>;
    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError>;
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError>;
    async fn get_variable(
        &mut self,
        name: String,
        recipe: Option<String>,
    ) -> Result<VariableValue, BackendError>;
    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError>;
    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError>;
    async fn get_signature_dump(
        &mut self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError>;
    async fn compare_signatures(
        &mut self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError>;
    async fn get_recipe_sources(&mut self, recipe: String) -> Result<Vec<PathBuf>, BackendError>;
    async fn get_recipe_metadata(&mut self, recipe: String)
    -> Result<RecipeMetadata, BackendError>;
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError>;
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError>;
    async fn cancel_build(&mut self) -> Result<(), BackendError>;
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError>;
    async fn shutdown(&mut self) -> Result<(), BackendError>;
}
