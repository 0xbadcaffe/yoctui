//! BitBake adapters. They execute BitBake; they never evaluate metadata themselves.
mod bitbake_cli_control;
mod bitbake_restart;
#[cfg(unix)]
mod bitbake_socket;
mod build_cache;
mod build_environment;
mod compatibility_api;
mod compatibility_cache;
mod compatibility_command;
mod compatibility_devtool;
#[cfg(any(test, feature = "test-fixtures"))]
mod compatibility_fixtures;
mod compatibility_layers;
mod compatibility_probe;
mod compatibility_recipetool;
mod compatibility_resolver;
mod compatibility_version;
mod image;
mod maintenance_optional;
mod maintenance_release;
mod maintenance_service;
mod maintenance_sstate;
mod package;
mod platform;
#[cfg(unix)]
mod pty_runner;
mod qa_layer;
mod qa_report;
mod qa_task;
mod qemu;
mod raw_job;
mod recipe_inventory;
mod rootfs;
mod sdk;
mod sdk_shell;
mod sdk_tool;
mod security;
mod security_mapper;
mod security_report;
mod server_controller;
mod signature;
mod test_results;
mod test_runner;
mod utility;
mod wic;

#[cfg(test)]
#[path = "tests/test_support/mod.rs"]
mod test_support;

use async_trait::async_trait;
pub use bitbake_cli_control::{
    BitBakeCliCommand, BitBakeCliControlError, BitBakeCliOperation, BitBakeCliOutcome,
    BitBakeCliPreview, BitBakeCliRunner,
};
pub use bitbake_restart::{
    BitBakeMetadataRefresher, BitBakeRestartCoordinator, BitBakeRestartError,
    BitBakeRestartMetadata,
};
#[cfg(unix)]
pub use bitbake_socket::BitBakeSocketAdapter;
pub use build_environment::{
    BuildEnvironmentAdapter, BuildEnvironmentAdapterError, BuildEnvironmentClonePreview,
    BuildEnvironmentResponse,
};
pub use compatibility_api::{
    BitBakeApiAuthority, BitBakeApiCompatibilityError, BitBakeApiOperation,
};
pub use compatibility_cache::{
    CapabilityCacheError, CapabilityCacheSelection, CapabilityFingerprintMaterial,
    CapabilitySnapshotCache,
};
pub use compatibility_command::{
    AuthorizedBitBakeCommand, BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
    BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION, BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION,
    BITBAKE_GETVAR_UTILITY_IMPLEMENTATION, BitBakeCommandAuthorizationError, BitBakeCommandPlanner,
    BitBakeServerCommandOperation,
};
pub use compatibility_devtool::{
    DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION, DEVTOOL_EDIT_RECIPE_IMPLEMENTATION,
    DEVTOOL_FINISH_IMPLEMENTATION, DEVTOOL_MODIFY_IMPLEMENTATION, DEVTOOL_RESET_IMPLEMENTATION,
    DEVTOOL_STATUS_IMPLEMENTATION, DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION,
    DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION, DEVTOOL_UPGRADE_IMPLEMENTATION, DevtoolCommandPlanner,
    DevtoolCompatibilityError,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use compatibility_fixtures::{
    CompatibilityFixtureRole, FixtureCapabilityExpectation, FixtureCapabilityState,
    ReleaseCapabilityFixture, fixture_implementation, fixture_state, release_capability_fixtures,
};
pub use compatibility_layers::{
    BITBAKE_LAYERS_ADD_IMPLEMENTATION, BITBAKE_LAYERS_CREATE_ADD_IMPLEMENTATION,
    BITBAKE_LAYERS_CREATE_IMPLEMENTATION, BITBAKE_LAYERS_CREATE_LAYERS_SETUP_IMPLEMENTATION,
    BITBAKE_LAYERS_FLATTEN_IMPLEMENTATION, BITBAKE_LAYERS_LAYERINDEX_FETCH_IMPLEMENTATION,
    BITBAKE_LAYERS_LAYERINDEX_SHOW_DEPENDS_IMPLEMENTATION, BITBAKE_LAYERS_REMOVE_IMPLEMENTATION,
    BITBAKE_LAYERS_SAVE_BUILD_CONF_IMPLEMENTATION, BITBAKE_LAYERS_SHOW_APPENDS_IMPLEMENTATION,
    BITBAKE_LAYERS_SHOW_CROSS_DEPENDS_IMPLEMENTATION, BITBAKE_LAYERS_SHOW_IMPLEMENTATION,
    BITBAKE_LAYERS_SHOW_MACHINES_IMPLEMENTATION, BITBAKE_LAYERS_SHOW_OVERLAYED_IMPLEMENTATION,
    BITBAKE_LAYERS_SHOW_RECIPES_IMPLEMENTATION, BitBakeLayersCommandPlanner,
    BitBakeLayersCommandSpec, BitBakeLayersCompatibilityError,
};
pub use compatibility_probe::{
    CapabilityProbeContext, CapabilityProbeContextError, CapabilityProbeObservation,
    CapabilityProbeRunner, CapabilityProbeStatus, probe_bundled_backend_capabilities,
};
pub use compatibility_recipetool::{
    RECIPETOOL_APPEND_FILE_IMPLEMENTATION, RECIPETOOL_CREATE_IMPLEMENTATION,
    RECIPETOOL_CREATE_OUTFILE_IMPLEMENTATION, RecipetoolCommandPlanner, RecipetoolCommandSpec,
    RecipetoolCompatibilityError,
};
pub use compatibility_resolver::{
    CapabilityResolver, ResolvedCapability, ResolvedCapabilitySnapshot,
};
pub use compatibility_version::{
    CorrelatedVersion, VersionFallbackMap, VersionFallbackResolution, VersionParseError,
};
pub use image::{
    ImageArtifactAdapter, ImageArtifactAdapterError, ImageArtifactCancellation,
    ImageArtifactResponse,
};
pub use maintenance_optional::{
    MaintenanceDirectoryIdentity, MaintenanceGitWorktreeIdentity, MaintenanceOptionalAdapterError,
    MaintenanceOptionalCapabilityInput, MaintenanceOptionalCapabilityInspector,
    MaintenanceOptionalInspection, OptionalErrorReportIntegration, OptionalIntegrationState,
    OptionalPullRequestIntegration, OptionalRepoManifestIntegration, OptionalToasterIntegration,
};
pub use maintenance_release::{
    GitArchiveLocalResult, MaintenanceReleaseAdapterError, MaintenanceReleaseCapabilityInput,
    MaintenanceReleaseCapabilityInspector, MaintenanceReleaseEvidenceSnapshot,
    build_compare_command, buildhistory_command, git_archive_local_command,
    git_archive_push_command, locked_signature_command,
};
pub use maintenance_service::{
    MaintenanceEndpointObservation, MaintenanceServiceAdapterError,
    MaintenanceServiceCapabilityInput, MaintenanceServiceCapabilityInspector,
    MaintenanceServiceInspection, pr_service_command,
};
pub use maintenance_sstate::{
    MaintenanceSstateAdapterError, MaintenanceSstateCapabilityInput,
    MaintenanceSstateCapabilityInspector, MaintenanceSstateCommandKind,
    MaintenanceSstateCommandSpec, MaintenanceSstateJobRunner, MaintenanceSstateRunnerEvent,
    parse_cleanup_preview,
};
pub use package::{
    PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION, PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
    PKGDATA_PACKAGE_INFO_IMPLEMENTATION, PKGDATA_READ_VALUE_IMPLEMENTATION, PackageDataAdapter,
    PackageDataAdapterError, PackageDataCancellation, PackageDataCommandSpec,
    PackageDetailResponse, PackageInventoryResponse,
};
pub use platform::{PlatformArtifactAdapter, PlatformArtifactAdapterError, PlatformArtifactScan};
#[cfg(unix)]
pub use pty_runner::{PtyRunner, PtyRunnerError, PtyRunnerEvent};
pub use qa_layer::{
    QaConfiguredLayerInput, QaLayerAdapterError, QaLayerCapabilityInput,
    QaLayerCapabilityInspector, QaLayerCapabilityResponse, QaLayerCommandSpec, QaLayerJobRunner,
    QaLayerRunnerEvent,
};
pub use qa_report::{
    QaReportAdapter, QaReportAdapterError, QaReportCancellation, QaReportCandidate, QaReportOrigin,
    QaReportResponse, QaReportScanInput, QaReportScanOutcome,
};
pub use qa_task::{
    QaFamilyTaskBinding, QaReportRootInput, QaTaskCapabilityError, QaTaskCapabilityInput,
    QaTaskCapabilityInspector, QaTaskCapabilityResponse, QaTaskScopeInput,
};
pub use qemu::{QemuAdapterError, QemuCapabilityInspector, QemuCommandSpec, QemuJobRunner};
pub use raw_job::{
    RawJobCommandSpec, RawJobPlanner, RawJobPlannerError, RawJobRunner, RawJobRunnerError,
    RawJobRunnerEvent, RawPtyCommandSpec, RawPtyPlanner,
};
pub use rootfs::{
    RootfsCompositionAdapter, RootfsCompositionAdapterError, RootfsCompositionCancellation,
    RootfsCompositionResponse, RootfsCompositionSources,
};
pub use sdk::{
    SdkArtifactAdapter, SdkArtifactAdapterError, SdkArtifactCancellation, SdkArtifactResponse,
    SdkArtifactScanOutcome,
};
pub use sdk_shell::{SdkShellAdapter, SdkShellEnvironment, SdkShellError, SdkShellPreview};
pub use sdk_tool::{
    SdkToolAdapter, SdkToolAdapterError, SdkToolCapabilityInspector, SdkToolCommandSpec,
    SdkToolJobRunner, SdkToolRunnerEvent,
};
pub use security::{SecurityCapabilityError, SecurityCapabilityInput, SecurityCapabilityInspector};
pub use security_mapper::{
    SecurityMapperAdapterError, SecurityMapperCommandSpec, SecurityMapperJobRunner,
    SecurityMapperRunnerEvent,
};
pub use security_report::{
    SecurityReportAdapter, SecurityReportAdapterError, SecurityReportCancellation,
    SecurityReportResponse, SecurityReportScanOutcome,
};
pub use server_controller::{
    BitBakeDetection, BitBakeServerAdapter, BitBakeServerAdapterError, BitBakeServerCapability,
    BitBakeServerContext, BitBakeServerController, BitBakeServerControllerError,
    BitBakeServerControllerState, BitBakeServerEndpoint, BitBakeServerLifecycle,
    BitBakeServerObservation, BitBakeServerOperation, BitBakeServerSession,
};
pub use signature::{
    SignatureAdapter, SignatureAdapterError, SignatureCancellation, SignatureCommandSpec,
    SignatureComparisonResponse, SignatureDumpResponse,
};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
pub use test_results::{
    ResultToolCapabilityInspector, TestResultAdapter, TestResultAdapterError,
    TestResultCommandSpec, TestResultImportResponse, TestResultJob, TestResultOperation,
    TestResultRunnerEvent,
};
pub use test_runner::{
    TestCommandSpec, TestRunnerAdapter, TestRunnerAdapterError, TestRunnerCapabilityInspector,
    TestRunnerEvent, TestRunnerJob,
};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command as TokioCommand},
};
pub use utility::{
    UtilityCommandSpec, UtilityCompatibilityAuthority, UtilityCompatibilityError, UtilityRisk,
    parse_utility_arguments,
};
pub use wic::{
    WicAdapterError, WicCapabilityInspector, WicCreateCommandSpec, WicDeviceInspector,
    WicDeviceInventoryResponse, WicJobRunner, WicWriteCommandSpec,
};
use yoctui_model::{
    BuildRequest, DependencyEdge, DependencyEdgeKind, DependencyGraph, DependencyNode,
    DependencyNodeId, DevtoolCapability, DevtoolGitState, DevtoolOperation, DevtoolStatus,
    DevtoolStatusError, DevtoolWorkspace, ImageArtifactInventory, ImageArtifactRequest, Layer,
    LogEntry, PackageDetail, PackageDetailRequest, PackageInventoryRequest, PackageSummary, Recipe,
    RecipeBuildStatus, RecipeIdentity, RecipeMetadata, RecipeWorkspaceStatus, RootfsComposition,
    RootfsCompositionRequest, Severity, SignatureComparisonRequest, SignatureDifference,
    SignatureRecord, SignatureTarget, TaskStats, VariableOperation, Workspace,
};
use yoctui_protocol::{
    Command, DependencyEdgeData, DependencyEdgeKindData, DependencyGraphData, DependencyNodeData,
    DependencyNodeIdData, Envelope, Event, LayerData, LayerRelationshipData, MAX_LINE_BYTES,
    ProtocolError, RecipeBuildStatusData, RecipeData, RecipeWorkspaceStatusData, TaskStatsData,
    VERSION, decode_line, encode_line,
};
pub use yoctui_utils::strip_ansi;
#[cfg(test)]
mod tests;

mod process_output;
pub use process_output::{BackendError, output_text};
use process_output::{
    DEPENDENCY_GRAPH_TIMEOUT, MAX_BRIDGE_STDERR_BYTES, MAX_DEPENDENCY_EDGES,
    MAX_DEPENDENCY_GRAPH_FILE_BYTES, MAX_DEPENDENCY_NODES, read_output,
};

mod devtool_runner;
pub use devtool_runner::{
    DevtoolCommandSpec, DevtoolInspector, DevtoolJobRunner, DevtoolOutputStream,
    DevtoolRunnerError, DevtoolRunnerEvent, QemuRunnerEvent, QemuRunnerOutputStream,
    WicRunnerEvent, WicRunnerOutputStream,
};

mod backend_api;
pub use backend_api::{
    BackendEvent, BitBakeBackend, DependencyGraphResponse, LayerRelationship, RecipeDependencies,
    VariableValue,
};

mod process_backend;
pub use process_backend::{ProcessBackend, classify_output};

mod bridge_backend;
use bridge_backend::{BUNDLED_BRIDGE_SOURCE, parse_task_dependency_dot};
pub use bridge_backend::{BridgeBackend, BridgeProcessPriority};

mod source_git;
pub use source_git::inspect_source_git;
