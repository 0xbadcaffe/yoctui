//! Effects.
use super::*;

pub(crate) fn next_filter<T: Clone + PartialEq>(values: &[T], current: Option<T>) -> Option<T> {
    let Some(current) = current else {
        return values.first().cloned();
    };
    values
        .iter()
        .position(|value| value == &current)
        .and_then(|index| values.get(index + 1))
        .cloned()
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    ReadEnvironmentDirectory {
        request: u64,
        path: PathBuf,
        initial: bool,
    },
    PersistSettings,
    PersistOnboarding,
    GenerateProjectProfile {
        profile: ProjectProfile,
        replace: bool,
    },
    VerifyBuildEnvironment {
        profile: BuildEnvironmentProfile,
        generation: u64,
    },
    CloneBuildEnvironment(BuildEnvironmentClonePlan),
    InspectKernel,
    InspectFirmware,
    Start(BuildRequest),
    Cancel,
    StartRaw(RawConfirmedExecutionRequest),
    CancelRaw(RawRequestId),
    SetRawAttachment {
        request: RawRequestId,
        attached: bool,
    },
    Terminal(TerminalEffect),
    LaunchDetachedTerminal(TerminalLaunchRequest),
    OpenInEditor(PathBuf),
    CopyToClipboard(String),
    OpenWorkspaceEditor {
        label: String,
        root: PathBuf,
    },
    LoadLayerBrowserDirectory {
        layer: String,
        root: PathBuf,
        directory: PathBuf,
    },
    LoadLayerBrowserPreview(PathBuf),
    OpenLayerBrowserEditor {
        layer: String,
        root: PathBuf,
        file: PathBuf,
    },
    DevtoolModify(RecipeIdentity),
    DevtoolReset(DevtoolResetPlan),
    DevtoolUpdateRecipe(RecipeIdentity),
    DevtoolFinish(DevtoolFinishPlan),
    DevtoolDeploy(DevtoolDeployPlan),
    InspectDevtoolStatus(RecipeIdentity),
    GetDependencies(String),
    GetSignatureDump(SignatureTarget),
    CompareSignatures(SignatureComparisonRequest),
    CancelSignatureOperation,
    GetPackageInventory(PackageInventoryRequest),
    GetPackageDetail(PackageDetailRequest),
    CancelPackageOperation,
    GetImageArtifacts(ImageArtifactRequest),
    CancelImageArtifactOperation,
    GetRootfsComposition(RootfsCompositionRequest),
    GetSdkArtifacts(SdkArtifactInventoryRequest),
    CancelSdkArtifactOperation,
    InspectSdkTools,
    StartSdkSession {
        id: SdkSessionId,
        operation: SdkOperation,
    },
    CancelSdkSession(SdkSessionId),
    InspectTestCapability,
    StartTestSession {
        id: TestSessionId,
        operation: TestOperation,
    },
    StartTestBuildSession {
        id: TestSessionId,
        family: TestFamily,
        request: BuildRequest,
    },
    CancelTestSession(TestSessionId),
    InspectResultToolCapability,
    ImportTestResults(TestResultImportRequest),
    CompareTestResults(TestComparisonRequest),
    InspectTestJunitDestination {
        result: TestResultIdentity,
        destination: PathBuf,
    },
    ExportTestJunit(TestJunitExportRequest),
    Security(SecurityEffect),
    Qa(QaEffect),
    Maintenance(MaintenanceEffect),
    InspectQemuCapability,
    StartQemuSession {
        id: QemuSessionId,
        request: QemuLaunchRequest,
    },
    CancelQemuSession(QemuSessionId),
    InspectWicCapability,
    GetWicOutputs(WicOutputInventoryRequest),
    GetWicDevices(WicDeviceInventoryRequest),
    StartWicSession {
        id: WicSessionId,
        operation: WicOperation,
    },
    CancelWicSession(WicSessionId),
    GetRecipeMetadata(String),
    GetVariable(VariableIdentity),
    WriteConfigAssignment(ConfigEditRequest),
    GetLayerRelationships,
    LoadRecipeEditorFile(PathBuf),
    SaveRecipeEditorFile {
        root: PathBuf,
        path: PathBuf,
        content: String,
        expected: TextAreaRevision,
    },
    WriteBbmask(String),
}
impl fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
