use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

pub const MAX_ENVIRONMENT_IDENTITY_TEXT_BYTES: usize = 1_024;
pub const MAX_ENVIRONMENT_IDENTITY_PATH_BYTES: usize = 4_096;
pub const MAX_ENVIRONMENT_SOURCE_ROOTS: usize = 256;
pub const MAX_ENVIRONMENT_LAYER_SERIES: usize = 256;
pub const MAX_ENVIRONMENT_TOOLS: usize = 256;
pub const MAX_LAYER_COMPATIBLE_SERIES: usize = 64;
pub const MAX_CAPABILITY_RECORDS: usize = 512;
pub const MAX_CAPABILITY_EVIDENCE: usize = 32;
pub const MAX_CAPABILITY_LIMITATIONS: usize = 32;
pub const MAX_CAPABILITY_EVIDENCE_ARGUMENTS: usize = 64;

/// Authoritative origins accepted by the environment identity model.
///
/// There is deliberately no branch-name, directory-name, nearest-tag, or
/// inferred-version variant. Those values may be diagnostics, but cannot
/// become authoritative identity through this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityAuthority {
    BackendHandshake,
    BitBakeDatastore,
    BitBakeVersionProbe,
    ConfiguredLayerMetadata,
    ExecutableProbe,
    InitializedEnvironment,
    ProtocolNegotiation,
    ReleaseMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AuthoritativeValue<T> {
    #[default]
    Unknown,
    Detected {
        value: T,
        authority: IdentityAuthority,
    },
}

impl<T> AuthoritativeValue<T> {
    pub const fn unknown() -> Self {
        Self::Unknown
    }

    pub const fn detected(value: T, authority: IdentityAuthority) -> Self {
        Self::Detected { value, authority }
    }

    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Unknown => None,
            Self::Detected { value, .. } => Some(value),
        }
    }

    pub const fn authority(&self) -> Option<IdentityAuthority> {
        match self {
            Self::Unknown => None,
            Self::Detected { authority, .. } => Some(*authority),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReleaseIdentity {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DistroIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRootKind {
    CoreBase,
    OpenEmbeddedCore,
    Poky,
    Layer,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceRootIdentity {
    pub kind: SourceRootKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LayerSeriesIdentity {
    pub layer: String,
    pub root: PathBuf,
    pub compatible_series: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ToolIdentity {
    pub id: String,
    pub executable: PathBuf,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BackendIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProtocolIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct YoctoEnvironmentIdentity {
    pub build_directory: AuthoritativeValue<PathBuf>,
    pub source_roots: AuthoritativeValue<Vec<SourceRootIdentity>>,
    pub bitbake_version: AuthoritativeValue<String>,
    pub oe_core: AuthoritativeValue<ReleaseIdentity>,
    pub poky: AuthoritativeValue<ReleaseIdentity>,
    pub distro: AuthoritativeValue<DistroIdentity>,
    pub machine: AuthoritativeValue<String>,
    pub layer_series: AuthoritativeValue<Vec<LayerSeriesIdentity>>,
    pub available_tools: AuthoritativeValue<Vec<ToolIdentity>>,
    pub backend: AuthoritativeValue<BackendIdentity>,
    pub protocol: AuthoritativeValue<ProtocolIdentity>,
}

impl YoctoEnvironmentIdentity {
    pub fn normalize(mut self) -> Result<Self, EnvironmentIdentityError> {
        validate_detected(
            "build_directory",
            &self.build_directory,
            &[
                IdentityAuthority::BackendHandshake,
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::InitializedEnvironment,
            ],
            |path| valid_absolute_path(path),
        )?;
        validate_detected(
            "bitbake_version",
            &self.bitbake_version,
            &[
                IdentityAuthority::BackendHandshake,
                IdentityAuthority::BitBakeVersionProbe,
            ],
            |value| valid_text(value),
        )?;
        validate_detected(
            "oe_core",
            &self.oe_core,
            &[
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::ConfiguredLayerMetadata,
                IdentityAuthority::ReleaseMetadata,
            ],
            valid_release,
        )?;
        validate_detected(
            "poky",
            &self.poky,
            &[
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::ConfiguredLayerMetadata,
                IdentityAuthority::ReleaseMetadata,
            ],
            valid_release,
        )?;
        validate_detected(
            "distro",
            &self.distro,
            &[IdentityAuthority::BitBakeDatastore],
            |value| valid_token(&value.name) && value.version.as_deref().is_none_or(valid_text),
        )?;
        validate_detected(
            "machine",
            &self.machine,
            &[IdentityAuthority::BitBakeDatastore],
            |value| valid_token(value),
        )?;
        validate_detected(
            "backend",
            &self.backend,
            &[IdentityAuthority::BackendHandshake],
            |value| valid_token(&value.name) && value.version.as_deref().is_none_or(valid_text),
        )?;
        validate_detected(
            "protocol",
            &self.protocol,
            &[IdentityAuthority::ProtocolNegotiation],
            |value| valid_token(&value.name) && valid_text(&value.version),
        )?;

        normalize_source_roots(&mut self.source_roots)?;
        normalize_layer_series(&mut self.layer_series)?;
        normalize_tools(&mut self.available_tools)?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CapabilityId {
    #[serde(rename = "bitbake.workspace_inspection")]
    BitBakeWorkspaceInspection,
    #[serde(rename = "bitbake.recipe_inventory")]
    BitBakeRecipeInventory,
    #[serde(rename = "bitbake.recipe_dependencies")]
    BitBakeRecipeDependencies,
    #[serde(rename = "bitbake.recipe_sources")]
    BitBakeRecipeSources,
    #[serde(rename = "bitbake.recipe_metadata")]
    BitBakeRecipeMetadata,
    #[serde(rename = "bitbake.layer_inventory")]
    BitBakeLayerInventory,
    #[serde(rename = "bitbake.layer_relationships")]
    BitBakeLayerRelationships,
    #[serde(rename = "bitbake.build")]
    BitBakeBuild,
    #[serde(rename = "bitbake.cancellation")]
    BitBakeCancellation,
    #[serde(rename = "bitbake.task_list")]
    BitBakeTaskList,
    #[serde(rename = "bitbake.force_task")]
    BitBakeForceTask,
    #[serde(rename = "bitbake.environment_dump")]
    BitBakeEnvironmentDump,
    #[serde(rename = "bitbake.graph_generation")]
    BitBakeGraphGeneration,
    #[serde(rename = "bitbake.dependency_graph")]
    BitBakeDependencyGraph,
    #[serde(rename = "bitbake.getvar")]
    BitBakeGetVar,
    #[serde(rename = "bitbake.variable_history")]
    BitBakeVariableHistory,
    #[serde(rename = "bitbake.diffsigs")]
    BitBakeDiffSigs,
    #[serde(rename = "bitbake.dumpsig")]
    BitBakeDumpSig,
    #[serde(rename = "bitbake.server_socket")]
    BitBakeServerSocket,
    #[serde(rename = "bitbake.server_status")]
    BitBakeServerStatus,
    #[serde(rename = "bitbake.server_start")]
    BitBakeServerStart,
    #[serde(rename = "bitbake.server_stop")]
    BitBakeServerStop,
    #[serde(rename = "bitbake.native_events")]
    BitBakeNativeEvents,
    #[serde(rename = "bitbake.raw.cli")]
    BitBakeRawCli,
    #[serde(rename = "bitbake.raw.show_versions")]
    BitBakeRawShowVersions,
    #[serde(rename = "bitbake.raw.task_execution")]
    BitBakeRawTaskExecution,
    #[serde(rename = "bitbake.raw.clear_stamp")]
    BitBakeRawClearStamp,
    #[serde(rename = "bitbake.raw.dry_run")]
    BitBakeRawDryRun,
    #[serde(rename = "bitbake.raw.parse_only")]
    BitBakeRawParseOnly,
    #[serde(rename = "bitbake.raw.continue")]
    BitBakeRawContinue,
    #[serde(rename = "bitbake.raw.profile")]
    BitBakeRawProfile,
    #[serde(rename = "bitbake.raw.dump_signatures")]
    BitBakeRawDumpSignatures,
    #[serde(rename = "bitbake.raw.revisions_changed")]
    BitBakeRawRevisionsChanged,
    #[serde(rename = "bitbake.raw.buildfile")]
    BitBakeRawBuildFile,
    #[serde(rename = "bitbake.raw.debug")]
    BitBakeRawDebug,
    #[serde(rename = "bitbake.raw.log_domains")]
    BitBakeRawLogDomains,
    #[serde(rename = "bitbake.raw.verbose")]
    BitBakeRawVerbose,
    #[serde(rename = "bitbake.raw.quiet")]
    BitBakeRawQuiet,
    #[serde(rename = "bitbake.raw.event_log")]
    BitBakeRawEventLog,
    #[serde(rename = "bitbake.raw.ui")]
    BitBakeRawUi,
    #[serde(rename = "bitbake.raw.server_bind")]
    BitBakeRawServerBind,
    #[serde(rename = "bitbake.raw.server_idle_timeout")]
    BitBakeRawServerIdleTimeout,
    #[serde(rename = "bitbake.raw.server_remote")]
    BitBakeRawServerRemote,
    #[serde(rename = "bitbake.raw.server_token")]
    BitBakeRawServerToken,
    #[serde(rename = "bitbake.raw.server_observe")]
    BitBakeRawServerObserve,
    #[serde(rename = "bitbake.raw.config_read")]
    BitBakeRawConfigRead,
    #[serde(rename = "bitbake.raw.config_postread")]
    BitBakeRawConfigPostRead,
    #[serde(rename = "bitbake.raw.ignore_deps")]
    BitBakeRawIgnoreDeps,
    #[serde(rename = "bitbake.raw.multiconfig")]
    BitBakeRawMulticonfig,
    #[serde(rename = "bitbake.raw.runall")]
    BitBakeRawRunAll,
    #[serde(rename = "bitbake.raw.runonly")]
    BitBakeRawRunOnly,
    #[serde(rename = "bitbake.raw.no_setscene")]
    BitBakeRawNoSetscene,
    #[serde(rename = "bitbake.raw.skip_setscene")]
    BitBakeRawSkipSetscene,
    #[serde(rename = "bitbake.raw.setscene_only")]
    BitBakeRawSetsceneOnly,
    #[serde(rename = "devtool.modify")]
    DevtoolModify,
    #[serde(rename = "devtool.status")]
    DevtoolStatus,
    #[serde(rename = "devtool.edit_recipe")]
    DevtoolEditRecipe,
    #[serde(rename = "devtool.update_recipe")]
    DevtoolUpdateRecipe,
    #[serde(rename = "devtool.finish")]
    DevtoolFinish,
    #[serde(rename = "devtool.deploy_target")]
    DevtoolDeployTarget,
    #[serde(rename = "devtool.undeploy_target")]
    DevtoolUndeployTarget,
    #[serde(rename = "devtool.reset")]
    DevtoolReset,
    #[serde(rename = "devtool.upgrade")]
    DevtoolUpgrade,
    #[serde(rename = "recipetool.create")]
    RecipetoolCreate,
    #[serde(rename = "recipetool.create_outfile")]
    RecipetoolCreateOutfile,
    #[serde(rename = "recipetool.appendfile")]
    RecipetoolAppendFile,
    #[serde(rename = "bitbake_layers.show_layers")]
    BitBakeLayersShowLayers,
    #[serde(rename = "bitbake_layers.create_layer")]
    BitBakeLayersCreateLayer,
    #[serde(rename = "bitbake_layers.create_and_add_layer")]
    BitBakeLayersCreateAndAddLayer,
    #[serde(rename = "bitbake_layers.add_layer")]
    BitBakeLayersAddLayer,
    #[serde(rename = "bitbake_layers.remove_layer")]
    BitBakeLayersRemoveLayer,
    #[serde(rename = "pkgdata.lookup_pkg")]
    PkgDataLookupPackage,
    #[serde(rename = "pkgdata.find_path")]
    PkgDataFindPath,
    #[serde(rename = "pkgdata.generated")]
    PkgDataGenerated,
    #[serde(rename = "pkgdata.list_packages")]
    PkgDataListPackages,
    #[serde(rename = "pkgdata.package_info")]
    PkgDataPackageInfo,
    #[serde(rename = "pkgdata.list_package_files")]
    PkgDataListPackageFiles,
    #[serde(rename = "pkgdata.read_value")]
    PkgDataReadValue,
    #[serde(rename = "wic.create")]
    WicCreate,
    #[serde(rename = "runqemu")]
    RunQemu,
    #[serde(rename = "sdk.populate")]
    SdkPopulate,
    #[serde(rename = "sdk.extensible")]
    SdkExtensible,
    #[serde(rename = "sdk.publish")]
    SdkPublish,
    #[serde(rename = "sdk.native_tools")]
    SdkNativeTools,
    #[serde(rename = "cve.check")]
    CveCheck,
    #[serde(rename = "spdx.create")]
    SpdxCreate,
    #[serde(rename = "yocto_check_layer")]
    YoctoCheckLayer,
    #[serde(rename = "resulttool")]
    ResultTool,
    #[serde(rename = "oe_selftest")]
    OeSelftest,
    #[serde(rename = "bitbake_selftest")]
    BitBakeSelftest,
    #[serde(rename = "testimage")]
    TestImage,
    #[serde(rename = "testsdk")]
    TestSdk,
    #[serde(rename = "testsdk_extensible")]
    TestSdkExtensible,
    #[serde(rename = "ptest")]
    Ptest,
    #[serde(rename = "qa.task")]
    QaTask,
    #[serde(rename = "menuconfig")]
    MenuConfig,
    #[serde(rename = "devshell")]
    DevShell,
    #[serde(rename = "buildhistory")]
    BuildHistory,
    #[serde(rename = "buildhistory.compare")]
    BuildHistoryCompare,
    #[serde(rename = "locked_signatures")]
    LockedSignatures,
    #[serde(rename = "hashserv.diagnostics")]
    HashservDiagnostics,
    #[serde(rename = "prserv.diagnostics")]
    PrservDiagnostics,
    #[serde(rename = "sstate.readiness")]
    SstateReadiness,
    #[serde(rename = "sstate.cleanup")]
    SstateCleanup,
    #[serde(rename = "prserv.management")]
    PrservManagement,
    #[serde(rename = "build_compare")]
    BuildCompare,
    #[serde(rename = "git_archive")]
    GitArchive,
}

impl CapabilityId {
    pub const ALL: [Self; 107] = [
        Self::BitBakeWorkspaceInspection,
        Self::BitBakeRecipeInventory,
        Self::BitBakeRecipeDependencies,
        Self::BitBakeRecipeSources,
        Self::BitBakeRecipeMetadata,
        Self::BitBakeLayerInventory,
        Self::BitBakeLayerRelationships,
        Self::BitBakeBuild,
        Self::BitBakeCancellation,
        Self::BitBakeTaskList,
        Self::BitBakeForceTask,
        Self::BitBakeEnvironmentDump,
        Self::BitBakeGraphGeneration,
        Self::BitBakeDependencyGraph,
        Self::BitBakeGetVar,
        Self::BitBakeVariableHistory,
        Self::BitBakeDiffSigs,
        Self::BitBakeDumpSig,
        Self::BitBakeServerSocket,
        Self::BitBakeServerStatus,
        Self::BitBakeServerStart,
        Self::BitBakeServerStop,
        Self::BitBakeNativeEvents,
        Self::BitBakeRawCli,
        Self::BitBakeRawShowVersions,
        Self::BitBakeRawTaskExecution,
        Self::BitBakeRawClearStamp,
        Self::BitBakeRawDryRun,
        Self::BitBakeRawParseOnly,
        Self::BitBakeRawContinue,
        Self::BitBakeRawProfile,
        Self::BitBakeRawDumpSignatures,
        Self::BitBakeRawRevisionsChanged,
        Self::BitBakeRawBuildFile,
        Self::BitBakeRawDebug,
        Self::BitBakeRawLogDomains,
        Self::BitBakeRawVerbose,
        Self::BitBakeRawQuiet,
        Self::BitBakeRawEventLog,
        Self::BitBakeRawUi,
        Self::BitBakeRawServerBind,
        Self::BitBakeRawServerIdleTimeout,
        Self::BitBakeRawServerRemote,
        Self::BitBakeRawServerToken,
        Self::BitBakeRawServerObserve,
        Self::BitBakeRawConfigRead,
        Self::BitBakeRawConfigPostRead,
        Self::BitBakeRawIgnoreDeps,
        Self::BitBakeRawMulticonfig,
        Self::BitBakeRawRunAll,
        Self::BitBakeRawRunOnly,
        Self::BitBakeRawNoSetscene,
        Self::BitBakeRawSkipSetscene,
        Self::BitBakeRawSetsceneOnly,
        Self::DevtoolModify,
        Self::DevtoolStatus,
        Self::DevtoolEditRecipe,
        Self::DevtoolUpdateRecipe,
        Self::DevtoolFinish,
        Self::DevtoolDeployTarget,
        Self::DevtoolUndeployTarget,
        Self::DevtoolReset,
        Self::DevtoolUpgrade,
        Self::RecipetoolCreate,
        Self::RecipetoolCreateOutfile,
        Self::RecipetoolAppendFile,
        Self::BitBakeLayersShowLayers,
        Self::BitBakeLayersCreateLayer,
        Self::BitBakeLayersCreateAndAddLayer,
        Self::BitBakeLayersAddLayer,
        Self::BitBakeLayersRemoveLayer,
        Self::PkgDataLookupPackage,
        Self::PkgDataFindPath,
        Self::PkgDataGenerated,
        Self::PkgDataListPackages,
        Self::PkgDataPackageInfo,
        Self::PkgDataListPackageFiles,
        Self::PkgDataReadValue,
        Self::WicCreate,
        Self::RunQemu,
        Self::SdkPopulate,
        Self::SdkExtensible,
        Self::SdkPublish,
        Self::SdkNativeTools,
        Self::CveCheck,
        Self::SpdxCreate,
        Self::YoctoCheckLayer,
        Self::ResultTool,
        Self::OeSelftest,
        Self::BitBakeSelftest,
        Self::TestImage,
        Self::TestSdk,
        Self::TestSdkExtensible,
        Self::Ptest,
        Self::QaTask,
        Self::MenuConfig,
        Self::DevShell,
        Self::BuildHistory,
        Self::BuildHistoryCompare,
        Self::LockedSignatures,
        Self::HashservDiagnostics,
        Self::PrservDiagnostics,
        Self::SstateReadiness,
        Self::SstateCleanup,
        Self::PrservManagement,
        Self::BuildCompare,
        Self::GitArchive,
    ];

    pub const RAW_CLI: [Self; 31] = [
        Self::BitBakeRawCli,
        Self::BitBakeRawShowVersions,
        Self::BitBakeRawTaskExecution,
        Self::BitBakeRawClearStamp,
        Self::BitBakeRawDryRun,
        Self::BitBakeRawParseOnly,
        Self::BitBakeRawContinue,
        Self::BitBakeRawProfile,
        Self::BitBakeRawDumpSignatures,
        Self::BitBakeRawRevisionsChanged,
        Self::BitBakeRawBuildFile,
        Self::BitBakeRawDebug,
        Self::BitBakeRawLogDomains,
        Self::BitBakeRawVerbose,
        Self::BitBakeRawQuiet,
        Self::BitBakeRawEventLog,
        Self::BitBakeRawUi,
        Self::BitBakeRawServerBind,
        Self::BitBakeRawServerIdleTimeout,
        Self::BitBakeRawServerRemote,
        Self::BitBakeRawServerToken,
        Self::BitBakeRawServerObserve,
        Self::BitBakeRawConfigRead,
        Self::BitBakeRawConfigPostRead,
        Self::BitBakeRawIgnoreDeps,
        Self::BitBakeRawMulticonfig,
        Self::BitBakeRawRunAll,
        Self::BitBakeRawRunOnly,
        Self::BitBakeRawNoSetscene,
        Self::BitBakeRawSkipSetscene,
        Self::BitBakeRawSetsceneOnly,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BitBakeWorkspaceInspection => "bitbake.workspace_inspection",
            Self::BitBakeRecipeInventory => "bitbake.recipe_inventory",
            Self::BitBakeRecipeDependencies => "bitbake.recipe_dependencies",
            Self::BitBakeRecipeSources => "bitbake.recipe_sources",
            Self::BitBakeRecipeMetadata => "bitbake.recipe_metadata",
            Self::BitBakeLayerInventory => "bitbake.layer_inventory",
            Self::BitBakeLayerRelationships => "bitbake.layer_relationships",
            Self::BitBakeBuild => "bitbake.build",
            Self::BitBakeCancellation => "bitbake.cancellation",
            Self::BitBakeTaskList => "bitbake.task_list",
            Self::BitBakeForceTask => "bitbake.force_task",
            Self::BitBakeEnvironmentDump => "bitbake.environment_dump",
            Self::BitBakeGraphGeneration => "bitbake.graph_generation",
            Self::BitBakeDependencyGraph => "bitbake.dependency_graph",
            Self::BitBakeGetVar => "bitbake.getvar",
            Self::BitBakeVariableHistory => "bitbake.variable_history",
            Self::BitBakeDiffSigs => "bitbake.diffsigs",
            Self::BitBakeDumpSig => "bitbake.dumpsig",
            Self::BitBakeServerSocket => "bitbake.server_socket",
            Self::BitBakeServerStatus => "bitbake.server_status",
            Self::BitBakeServerStart => "bitbake.server_start",
            Self::BitBakeServerStop => "bitbake.server_stop",
            Self::BitBakeNativeEvents => "bitbake.native_events",
            Self::BitBakeRawCli => "bitbake.raw.cli",
            Self::BitBakeRawShowVersions => "bitbake.raw.show_versions",
            Self::BitBakeRawTaskExecution => "bitbake.raw.task_execution",
            Self::BitBakeRawClearStamp => "bitbake.raw.clear_stamp",
            Self::BitBakeRawDryRun => "bitbake.raw.dry_run",
            Self::BitBakeRawParseOnly => "bitbake.raw.parse_only",
            Self::BitBakeRawContinue => "bitbake.raw.continue",
            Self::BitBakeRawProfile => "bitbake.raw.profile",
            Self::BitBakeRawDumpSignatures => "bitbake.raw.dump_signatures",
            Self::BitBakeRawRevisionsChanged => "bitbake.raw.revisions_changed",
            Self::BitBakeRawBuildFile => "bitbake.raw.buildfile",
            Self::BitBakeRawDebug => "bitbake.raw.debug",
            Self::BitBakeRawLogDomains => "bitbake.raw.log_domains",
            Self::BitBakeRawVerbose => "bitbake.raw.verbose",
            Self::BitBakeRawQuiet => "bitbake.raw.quiet",
            Self::BitBakeRawEventLog => "bitbake.raw.event_log",
            Self::BitBakeRawUi => "bitbake.raw.ui",
            Self::BitBakeRawServerBind => "bitbake.raw.server_bind",
            Self::BitBakeRawServerIdleTimeout => "bitbake.raw.server_idle_timeout",
            Self::BitBakeRawServerRemote => "bitbake.raw.server_remote",
            Self::BitBakeRawServerToken => "bitbake.raw.server_token",
            Self::BitBakeRawServerObserve => "bitbake.raw.server_observe",
            Self::BitBakeRawConfigRead => "bitbake.raw.config_read",
            Self::BitBakeRawConfigPostRead => "bitbake.raw.config_postread",
            Self::BitBakeRawIgnoreDeps => "bitbake.raw.ignore_deps",
            Self::BitBakeRawMulticonfig => "bitbake.raw.multiconfig",
            Self::BitBakeRawRunAll => "bitbake.raw.runall",
            Self::BitBakeRawRunOnly => "bitbake.raw.runonly",
            Self::BitBakeRawNoSetscene => "bitbake.raw.no_setscene",
            Self::BitBakeRawSkipSetscene => "bitbake.raw.skip_setscene",
            Self::BitBakeRawSetsceneOnly => "bitbake.raw.setscene_only",
            Self::DevtoolModify => "devtool.modify",
            Self::DevtoolStatus => "devtool.status",
            Self::DevtoolEditRecipe => "devtool.edit_recipe",
            Self::DevtoolUpdateRecipe => "devtool.update_recipe",
            Self::DevtoolFinish => "devtool.finish",
            Self::DevtoolDeployTarget => "devtool.deploy_target",
            Self::DevtoolUndeployTarget => "devtool.undeploy_target",
            Self::DevtoolReset => "devtool.reset",
            Self::DevtoolUpgrade => "devtool.upgrade",
            Self::RecipetoolCreate => "recipetool.create",
            Self::RecipetoolCreateOutfile => "recipetool.create_outfile",
            Self::RecipetoolAppendFile => "recipetool.appendfile",
            Self::BitBakeLayersShowLayers => "bitbake_layers.show_layers",
            Self::BitBakeLayersCreateLayer => "bitbake_layers.create_layer",
            Self::BitBakeLayersCreateAndAddLayer => "bitbake_layers.create_and_add_layer",
            Self::BitBakeLayersAddLayer => "bitbake_layers.add_layer",
            Self::BitBakeLayersRemoveLayer => "bitbake_layers.remove_layer",
            Self::PkgDataLookupPackage => "pkgdata.lookup_pkg",
            Self::PkgDataFindPath => "pkgdata.find_path",
            Self::PkgDataGenerated => "pkgdata.generated",
            Self::PkgDataListPackages => "pkgdata.list_packages",
            Self::PkgDataPackageInfo => "pkgdata.package_info",
            Self::PkgDataListPackageFiles => "pkgdata.list_package_files",
            Self::PkgDataReadValue => "pkgdata.read_value",
            Self::WicCreate => "wic.create",
            Self::RunQemu => "runqemu",
            Self::SdkPopulate => "sdk.populate",
            Self::SdkExtensible => "sdk.extensible",
            Self::SdkPublish => "sdk.publish",
            Self::SdkNativeTools => "sdk.native_tools",
            Self::CveCheck => "cve.check",
            Self::SpdxCreate => "spdx.create",
            Self::YoctoCheckLayer => "yocto_check_layer",
            Self::ResultTool => "resulttool",
            Self::OeSelftest => "oe_selftest",
            Self::BitBakeSelftest => "bitbake_selftest",
            Self::TestImage => "testimage",
            Self::TestSdk => "testsdk",
            Self::TestSdkExtensible => "testsdk_extensible",
            Self::Ptest => "ptest",
            Self::QaTask => "qa.task",
            Self::MenuConfig => "menuconfig",
            Self::DevShell => "devshell",
            Self::BuildHistory => "buildhistory",
            Self::BuildHistoryCompare => "buildhistory.compare",
            Self::LockedSignatures => "locked_signatures",
            Self::HashservDiagnostics => "hashserv.diagnostics",
            Self::PrservDiagnostics => "prserv.diagnostics",
            Self::SstateReadiness => "sstate.readiness",
            Self::SstateCleanup => "sstate.cleanup",
            Self::PrservManagement => "prserv.management",
            Self::BuildCompare => "build_compare",
            Self::GitArchive => "git_archive",
        }
    }

    pub fn from_stable_name(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.as_str() == value)
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CapabilityReasonCode(String);

impl CapabilityReasonCode {
    pub fn new(value: impl Into<String>) -> Result<Self, CapabilityModelError> {
        let value = value.into();
        if !valid_reason_code(&value) {
            return Err(CapabilityModelError::InvalidReasonCode(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReason {
    pub code: CapabilityReasonCode,
    pub message: String,
    pub requirement: Option<String>,
}

impl CapabilityReason {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        requirement: Option<String>,
    ) -> Result<Self, CapabilityModelError> {
        let reason = Self {
            code: CapabilityReasonCode::new(code)?,
            message: message.into(),
            requirement,
        };
        reason.validate()?;
        Ok(reason)
    }

    fn validate(&self) -> Result<(), CapabilityModelError> {
        if !valid_reason_code(self.code.as_str())
            || !valid_text(&self.message)
            || self
                .requirement
                .as_deref()
                .is_some_and(|value| !valid_text(value))
        {
            return Err(CapabilityModelError::InvalidReason);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidenceKind {
    DirectProbe,
    BackendNegotiation,
    ProtocolNegotiation,
    Metadata,
    ExecutableIdentity,
    ReleaseVersionFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidenceOutcome {
    Positive,
    Negative,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEvidence {
    pub kind: CapabilityEvidenceKind,
    pub outcome: CapabilityEvidenceOutcome,
    pub subject: String,
    pub detail: String,
    #[serde(default)]
    pub argv: Vec<String>,
}

impl CapabilityEvidence {
    fn validate(&self) -> Result<(), CapabilityModelError> {
        if !valid_text(&self.subject)
            || !valid_text(&self.detail)
            || self.argv.len() > MAX_CAPABILITY_EVIDENCE_ARGUMENTS
            || self.argv.iter().any(|argument| !valid_text(argument))
        {
            return Err(CapabilityModelError::InvalidEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    AvailableWithLimitations {
        reason: CapabilityReason,
        limitations: Vec<String>,
    },
    Unavailable {
        reason: CapabilityReason,
    },
    Unknown {
        reason: CapabilityReason,
    },
    Unsupported {
        reason: CapabilityReason,
    },
}

impl CapabilityState {
    pub const fn is_enabled(&self) -> bool {
        matches!(
            self,
            Self::Available | Self::AvailableWithLimitations { .. }
        )
    }

    pub const fn reason(&self) -> Option<&CapabilityReason> {
        match self {
            Self::Available => None,
            Self::AvailableWithLimitations { reason, .. }
            | Self::Unavailable { reason }
            | Self::Unknown { reason }
            | Self::Unsupported { reason } => Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRecord {
    pub id: CapabilityId,
    pub state: CapabilityState,
    pub evidence: Vec<CapabilityEvidence>,
}

impl CapabilityRecord {
    fn normalize(&mut self) -> Result<(), CapabilityModelError> {
        if self.evidence.len() > MAX_CAPABILITY_EVIDENCE {
            return Err(CapabilityModelError::TooMuchEvidence {
                id: self.id,
                count: self.evidence.len(),
            });
        }
        if self
            .evidence
            .iter()
            .any(|evidence| evidence.validate().is_err())
        {
            return Err(CapabilityModelError::InvalidEvidence);
        }
        match &mut self.state {
            CapabilityState::Available => {
                require_evidence(self.id, &self.evidence, CapabilityEvidenceOutcome::Positive)?
            }
            CapabilityState::AvailableWithLimitations {
                reason,
                limitations,
            } => {
                reason.validate()?;
                if limitations.is_empty()
                    || limitations.len() > MAX_CAPABILITY_LIMITATIONS
                    || limitations.iter().any(|limitation| !valid_text(limitation))
                {
                    return Err(CapabilityModelError::InvalidLimitations(self.id));
                }
                limitations.sort();
                limitations.dedup();
                require_evidence(self.id, &self.evidence, CapabilityEvidenceOutcome::Positive)?;
            }
            CapabilityState::Unavailable { reason } => {
                reason.validate()?;
                require_evidence(self.id, &self.evidence, CapabilityEvidenceOutcome::Negative)?;
            }
            CapabilityState::Unknown { reason } | CapabilityState::Unsupported { reason } => {
                reason.validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub generation: u64,
    pub environment: YoctoEnvironmentIdentity,
    pub capabilities: Vec<CapabilityRecord>,
}

impl CapabilitySnapshot {
    pub fn normalize(mut self) -> Result<Self, CapabilityModelError> {
        if self.generation == 0 {
            return Err(CapabilityModelError::InvalidGeneration);
        }
        self.environment = self.environment.normalize()?;
        if self.capabilities.len() > MAX_CAPABILITY_RECORDS {
            return Err(CapabilityModelError::TooManyCapabilities(
                self.capabilities.len(),
            ));
        }
        let mut seen = BTreeSet::new();
        for capability in &mut self.capabilities {
            if !seen.insert(capability.id) {
                return Err(CapabilityModelError::DuplicateCapability(capability.id));
            }
            capability.normalize()?;
        }
        self.capabilities.sort_by_key(|capability| capability.id);
        Ok(self)
    }

    pub fn capability(&self, id: CapabilityId) -> Option<&CapabilityRecord> {
        self.capabilities
            .binary_search_by_key(&id, |capability| capability.id)
            .ok()
            .map(|index| &self.capabilities[index])
    }

    pub fn allows(&self, id: CapabilityId) -> bool {
        self.capability(id)
            .is_some_and(|capability| capability.state.is_enabled())
    }

    pub fn availability_summary(&self) -> CapabilityAvailabilitySummary {
        let mut summary = CapabilityAvailabilitySummary::default();
        for capability in &self.capabilities {
            match capability.state {
                CapabilityState::Available => summary.available += 1,
                CapabilityState::AvailableWithLimitations { .. } => summary.limited += 1,
                CapabilityState::Unavailable { .. } => summary.unavailable += 1,
                CapabilityState::Unknown { .. } => summary.unknown += 1,
                CapabilityState::Unsupported { .. } => summary.unsupported += 1,
            }
        }
        summary
    }

    pub fn operating_mode(&self) -> EnvironmentOperatingMode {
        let summary = self.availability_summary();
        if summary.unavailable == 0
            && summary.unknown == 0
            && summary.unsupported == 0
            && summary.limited == 0
        {
            EnvironmentOperatingMode::Full
        } else if summary.available + summary.limited > 0 {
            EnvironmentOperatingMode::Degraded
        } else {
            EnvironmentOperatingMode::Diagnostic
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CapabilityAvailabilitySummary {
    pub available: usize,
    pub limited: usize,
    pub unavailable: usize,
    pub unknown: usize,
    pub unsupported: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentOperatingMode {
    Full,
    Degraded,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCacheKey {
    pub environment: YoctoEnvironmentIdentity,
    pub workspace_identity: String,
    pub initialized_environment_digest: String,
    pub layer_configuration_digest: String,
    pub build_configuration_digest: String,
    pub daemon_workspace_identity: String,
}

impl CapabilityCacheKey {
    pub fn normalize(mut self) -> Result<Self, CapabilityCacheKeyError> {
        self.environment = self.environment.normalize()?;
        if !valid_text(&self.workspace_identity)
            || !valid_text(&self.daemon_workspace_identity)
            || !valid_digest(&self.initialized_environment_digest)
            || !valid_digest(&self.layer_configuration_digest)
            || !valid_digest(&self.build_configuration_digest)
        {
            return Err(CapabilityCacheKeyError::InvalidField);
        }
        Ok(self)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityCacheKeyError {
    #[error(transparent)]
    InvalidEnvironment(#[from] EnvironmentIdentityError),
    #[error("capability cache key contains an invalid field")]
    InvalidField,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityModelError {
    #[error(transparent)]
    InvalidEnvironment(#[from] EnvironmentIdentityError),
    #[error("capability snapshot generation must be non-zero")]
    InvalidGeneration,
    #[error("too many capabilities in snapshot: {0}")]
    TooManyCapabilities(usize),
    #[error("duplicate capability in snapshot: {0}")]
    DuplicateCapability(CapabilityId),
    #[error("invalid capability reason code: {0}")]
    InvalidReasonCode(String),
    #[error("invalid capability reason")]
    InvalidReason,
    #[error("invalid capability evidence")]
    InvalidEvidence,
    #[error("too much evidence for capability {id}: {count}")]
    TooMuchEvidence { id: CapabilityId, count: usize },
    #[error("capability {id} lacks required {outcome:?} evidence")]
    MissingEvidence {
        id: CapabilityId,
        outcome: CapabilityEvidenceOutcome,
    },
    #[error("invalid limitations for capability {0}")]
    InvalidLimitations(CapabilityId),
}

fn require_evidence(
    id: CapabilityId,
    evidence: &[CapabilityEvidence],
    outcome: CapabilityEvidenceOutcome,
) -> Result<(), CapabilityModelError> {
    if evidence.iter().any(|item| item.outcome == outcome) {
        Ok(())
    } else {
        Err(CapabilityModelError::MissingEvidence { id, outcome })
    }
}

fn valid_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'.'
        })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EnvironmentIdentityError {
    #[error("invalid authoritative environment identity field: {0}")]
    InvalidField(&'static str),
    #[error("invalid authority {authority:?} for environment identity field {field}")]
    InvalidAuthority {
        field: &'static str,
        authority: IdentityAuthority,
    },
    #[error("too many entries in environment identity field {field}: {count} > {limit}")]
    TooManyEntries {
        field: &'static str,
        count: usize,
        limit: usize,
    },
    #[error("conflicting duplicate environment identity for {field}: {key}")]
    ConflictingDuplicate { field: &'static str, key: String },
}

fn validate_detected<T>(
    field: &'static str,
    detected: &AuthoritativeValue<T>,
    authorities: &[IdentityAuthority],
    valid: impl FnOnce(&T) -> bool,
) -> Result<(), EnvironmentIdentityError> {
    let AuthoritativeValue::Detected { value, authority } = detected else {
        return Ok(());
    };
    if !authorities.contains(authority) {
        return Err(EnvironmentIdentityError::InvalidAuthority {
            field,
            authority: *authority,
        });
    }
    if !valid(value) {
        return Err(EnvironmentIdentityError::InvalidField(field));
    }
    Ok(())
}

fn normalize_source_roots(
    value: &mut AuthoritativeValue<Vec<SourceRootIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "source_roots",
        value,
        &[
            IdentityAuthority::BitBakeDatastore,
            IdentityAuthority::ConfiguredLayerMetadata,
            IdentityAuthority::InitializedEnvironment,
        ],
        |roots| {
            !roots.is_empty()
                && roots.len() <= MAX_ENVIRONMENT_SOURCE_ROOTS
                && roots.iter().all(|root| {
                    valid_absolute_path(&root.path)
                        && match &root.kind {
                            SourceRootKind::Other(label) => valid_token(label),
                            _ => true,
                        }
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: roots, .. } = value else {
        return Ok(());
    };
    roots.sort();
    roots.dedup();
    Ok(())
}

fn normalize_layer_series(
    value: &mut AuthoritativeValue<Vec<LayerSeriesIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "layer_series",
        value,
        &[IdentityAuthority::ConfiguredLayerMetadata],
        |layers| {
            !layers.is_empty()
                && layers.len() <= MAX_ENVIRONMENT_LAYER_SERIES
                && layers.iter().all(|layer| {
                    valid_token(&layer.layer)
                        && valid_absolute_path(&layer.root)
                        && !layer.compatible_series.is_empty()
                        && layer.compatible_series.len() <= MAX_LAYER_COMPATIBLE_SERIES
                        && layer
                            .compatible_series
                            .iter()
                            .all(|series| valid_token(series))
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: layers, .. } = value else {
        return Ok(());
    };
    for layer in layers.iter_mut() {
        layer.compatible_series.sort();
        layer.compatible_series.dedup();
    }
    reject_conflicts(
        "layer_series",
        layers,
        |layer| layer.layer.clone(),
        |layer| (layer.root.clone(), layer.compatible_series.clone()),
    )?;
    layers.sort();
    layers.dedup();
    Ok(())
}

fn normalize_tools(
    value: &mut AuthoritativeValue<Vec<ToolIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "available_tools",
        value,
        &[
            IdentityAuthority::ExecutableProbe,
            IdentityAuthority::InitializedEnvironment,
        ],
        |tools| {
            !tools.is_empty()
                && tools.len() <= MAX_ENVIRONMENT_TOOLS
                && tools.iter().all(|tool| {
                    valid_token(&tool.id)
                        && valid_absolute_path(&tool.executable)
                        && tool.version.as_deref().is_none_or(valid_text)
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: tools, .. } = value else {
        return Ok(());
    };
    reject_conflicts(
        "available_tools",
        tools,
        |tool| tool.id.clone(),
        |tool| (tool.executable.clone(), tool.version.clone()),
    )?;
    tools.sort();
    tools.dedup();
    Ok(())
}

fn reject_conflicts<T, V: PartialEq>(
    field: &'static str,
    values: &[T],
    key: impl Fn(&T) -> String,
    identity: impl Fn(&T) -> V,
) -> Result<(), EnvironmentIdentityError> {
    let mut seen = BTreeMap::new();
    for value in values {
        let key = key(value);
        let identity = identity(value);
        if seen.get(&key).is_some_and(|previous| previous != &identity) {
            return Err(EnvironmentIdentityError::ConflictingDuplicate { field, key });
        }
        seen.insert(key, identity);
    }
    Ok(())
}

fn valid_release(value: &ReleaseIdentity) -> bool {
    (value.name.is_some() || value.version.is_some())
        && value.name.as_deref().is_none_or(valid_text)
        && value.version.as_deref().is_none_or(valid_text)
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ENVIRONMENT_IDENTITY_TEXT_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_token(value: &str) -> bool {
    valid_text(value) && !value.chars().any(char::is_whitespace)
}

fn valid_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().as_encoded_bytes().len() <= MAX_ENVIRONMENT_IDENTITY_PATH_BYTES
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

#[cfg(test)]
mod capability {
    use super::*;

    fn environment() -> YoctoEnvironmentIdentity {
        YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                "/work/build".into(),
                IdentityAuthority::InitializedEnvironment,
            ),
            bitbake_version: AuthoritativeValue::detected(
                "2.8.1".into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
    }

    fn reason(code: &str) -> CapabilityReason {
        CapabilityReason::new(code, "Exact environment-correlated reason", None).unwrap()
    }

    fn evidence(outcome: CapabilityEvidenceOutcome) -> CapabilityEvidence {
        CapabilityEvidence {
            kind: CapabilityEvidenceKind::DirectProbe,
            outcome,
            subject: "devtool --help".into(),
            detail: "Bounded help probe inspected the upgrade subcommand".into(),
            argv: vec!["/work/bin/devtool".into(), "--help".into()],
        }
    }

    fn record(id: CapabilityId, state: CapabilityState) -> CapabilityRecord {
        let outcome = match state {
            CapabilityState::Available | CapabilityState::AvailableWithLimitations { .. } => {
                CapabilityEvidenceOutcome::Positive
            }
            CapabilityState::Unavailable { .. } => CapabilityEvidenceOutcome::Negative,
            CapabilityState::Unknown { .. } | CapabilityState::Unsupported { .. } => {
                CapabilityEvidenceOutcome::Inconclusive
            }
        };
        CapabilityRecord {
            id,
            state,
            evidence: vec![evidence(outcome)],
        }
    }

    #[test]
    fn capability_snapshot_normalizes_all_states_and_supports_fail_closed_lookup() {
        let snapshot = CapabilitySnapshot {
            generation: 7,
            environment: environment(),
            capabilities: vec![
                record(
                    CapabilityId::DevtoolUpgrade,
                    CapabilityState::Unavailable {
                        reason: reason("command.missing"),
                    },
                ),
                record(CapabilityId::BitBakeGetVar, CapabilityState::Available),
                record(
                    CapabilityId::SpdxCreate,
                    CapabilityState::AvailableWithLimitations {
                        reason: reason("fallback.legacy_spdx"),
                        limitations: vec![
                            "Legacy SPDX task does not emit the newest schema".into(),
                        ],
                    },
                ),
                record(
                    CapabilityId::ResultTool,
                    CapabilityState::Unknown {
                        reason: reason("probe.not_run"),
                    },
                ),
                record(
                    CapabilityId::RecipetoolAppendFile,
                    CapabilityState::Unsupported {
                        reason: reason("yoctui.no_safe_implementation"),
                    },
                ),
            ],
        }
        .normalize()
        .unwrap();

        assert_eq!(snapshot.generation, 7);
        assert!(snapshot.allows(CapabilityId::BitBakeGetVar));
        assert!(snapshot.allows(CapabilityId::SpdxCreate));
        assert!(!snapshot.allows(CapabilityId::DevtoolUpgrade));
        assert!(!snapshot.allows(CapabilityId::ResultTool));
        assert!(!snapshot.allows(CapabilityId::RecipetoolAppendFile));
        assert!(!snapshot.allows(CapabilityId::RunQemu));
        assert_eq!(
            snapshot
                .capability(CapabilityId::DevtoolUpgrade)
                .unwrap()
                .state
                .reason()
                .unwrap()
                .code
                .as_str(),
            "command.missing"
        );
        assert!(
            snapshot
                .capabilities
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
        );
    }

    #[test]
    fn capability_inventory_is_unique_behavior_oriented_and_complete() {
        let ids = CapabilityId::ALL
            .into_iter()
            .map(CapabilityId::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), CapabilityId::ALL.len());
        assert!(ids.contains("bitbake.force_task"));
        assert!(ids.contains("devtool.upgrade"));
        assert!(ids.contains("recipetool.appendfile"));
        assert!(ids.contains("pkgdata.find_path"));
        assert!(ids.contains("hashserv.diagnostics"));
        assert!(ids.iter().all(|id| !id.chars().any(char::is_whitespace)));
        assert!(
            ids.iter()
                .all(|id| !id.chars().any(|ch| ch.is_ascii_digit()))
        );
    }

    #[test]
    fn capability_snapshot_rejects_duplicates_and_wrong_evidence_polarity() {
        let duplicate = record(CapabilityId::WicCreate, CapabilityState::Available);
        let error = CapabilitySnapshot {
            generation: 1,
            environment: environment(),
            capabilities: vec![duplicate.clone(), duplicate],
        }
        .normalize()
        .unwrap_err();
        assert_eq!(
            error,
            CapabilityModelError::DuplicateCapability(CapabilityId::WicCreate)
        );

        let error = CapabilitySnapshot {
            generation: 1,
            environment: environment(),
            capabilities: vec![CapabilityRecord {
                id: CapabilityId::DevtoolUpgrade,
                state: CapabilityState::Unavailable {
                    reason: reason("command.missing"),
                },
                evidence: vec![evidence(CapabilityEvidenceOutcome::Positive)],
            }],
        }
        .normalize()
        .unwrap_err();
        assert_eq!(
            error,
            CapabilityModelError::MissingEvidence {
                id: CapabilityId::DevtoolUpgrade,
                outcome: CapabilityEvidenceOutcome::Negative,
            }
        );
    }

    #[test]
    fn capability_snapshot_rejects_invalid_generation_reason_evidence_and_limitations() {
        let error = CapabilitySnapshot {
            generation: 0,
            environment: environment(),
            capabilities: Vec::new(),
        }
        .normalize()
        .unwrap_err();
        assert_eq!(error, CapabilityModelError::InvalidGeneration);

        assert!(CapabilityReasonCode::new("Command Missing").is_err());
        assert!(CapabilityReason::new("command.missing", "bad\nreason", None).is_err());

        let mut bad_evidence = evidence(CapabilityEvidenceOutcome::Positive);
        bad_evidence.argv = vec!["x".repeat(MAX_ENVIRONMENT_IDENTITY_TEXT_BYTES + 1)];
        let error = CapabilitySnapshot {
            generation: 1,
            environment: environment(),
            capabilities: vec![CapabilityRecord {
                id: CapabilityId::BitBakeBuild,
                state: CapabilityState::Available,
                evidence: vec![bad_evidence],
            }],
        }
        .normalize()
        .unwrap_err();
        assert_eq!(error, CapabilityModelError::InvalidEvidence);

        let error = CapabilitySnapshot {
            generation: 1,
            environment: environment(),
            capabilities: vec![record(
                CapabilityId::SpdxCreate,
                CapabilityState::AvailableWithLimitations {
                    reason: reason("fallback.legacy_spdx"),
                    limitations: Vec::new(),
                },
            )],
        }
        .normalize()
        .unwrap_err();
        assert_eq!(
            error,
            CapabilityModelError::InvalidLimitations(CapabilityId::SpdxCreate)
        );
    }

    #[test]
    fn capability_snapshot_keeps_exact_environment_association() {
        let first = CapabilitySnapshot {
            generation: 1,
            environment: environment(),
            capabilities: Vec::new(),
        }
        .normalize()
        .unwrap();
        let mut other_environment = environment();
        other_environment.build_directory = AuthoritativeValue::detected(
            "/other/build".into(),
            IdentityAuthority::InitializedEnvironment,
        );
        let second = CapabilitySnapshot {
            generation: 2,
            environment: other_environment,
            capabilities: Vec::new(),
        }
        .normalize()
        .unwrap();
        assert_ne!(first.environment, second.environment);
        assert_eq!(
            first.environment.build_directory.value(),
            Some(&PathBuf::from("/work/build"))
        );
        assert_eq!(
            second.environment.build_directory.value(),
            Some(&PathBuf::from("/other/build"))
        );
    }

    #[test]
    fn compatibility_older_release_preserves_mixed_state_without_global_failure() {
        let snapshot = CapabilitySnapshot {
            generation: 8,
            environment: environment(),
            capabilities: vec![
                record(
                    CapabilityId::BitBakeWorkspaceInspection,
                    CapabilityState::Available,
                ),
                record(
                    CapabilityId::BitBakeNativeEvents,
                    CapabilityState::AvailableWithLimitations {
                        reason: reason("fallback.version_inference"),
                        limitations: vec!["Legacy Tinfoil adapter is selected".into()],
                    },
                ),
                record(
                    CapabilityId::DevtoolUpgrade,
                    CapabilityState::Unavailable {
                        reason: reason("command.missing"),
                    },
                ),
                record(
                    CapabilityId::ResultTool,
                    CapabilityState::Unknown {
                        reason: reason("probe.not_run"),
                    },
                ),
                record(
                    CapabilityId::SpdxCreate,
                    CapabilityState::Unsupported {
                        reason: reason("yoctui.no_safe_implementation"),
                    },
                ),
            ],
        }
        .normalize()
        .unwrap();
        assert_eq!(
            snapshot.operating_mode(),
            EnvironmentOperatingMode::Degraded
        );
        assert_eq!(
            snapshot.availability_summary(),
            CapabilityAvailabilitySummary {
                available: 1,
                limited: 1,
                unavailable: 1,
                unknown: 1,
                unsupported: 1,
            }
        );
        assert!(snapshot.allows(CapabilityId::BitBakeWorkspaceInspection));
        assert!(snapshot.allows(CapabilityId::BitBakeNativeEvents));
        assert!(!snapshot.allows(CapabilityId::DevtoolUpgrade));

        let diagnostic = CapabilitySnapshot {
            generation: 9,
            environment: environment(),
            capabilities: vec![record(
                CapabilityId::BitBakeBuild,
                CapabilityState::Unknown {
                    reason: reason("probe.not_run"),
                },
            )],
        }
        .normalize()
        .unwrap();
        assert_eq!(
            diagnostic.operating_mode(),
            EnvironmentOperatingMode::Diagnostic
        );
    }
}

#[cfg(test)]
mod compatibility_cache {
    use super::*;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn key() -> CapabilityCacheKey {
        CapabilityCacheKey {
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    "/workspace/build".into(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                bitbake_version: AuthoritativeValue::detected(
                    "2.8.1".into(),
                    IdentityAuthority::BitBakeVersionProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            workspace_identity: "/workspace/poky@0123456789abcdef".into(),
            initialized_environment_digest: digest('a'),
            layer_configuration_digest: digest('b'),
            build_configuration_digest: digest('c'),
            daemon_workspace_identity: "daemon-workspace-one".into(),
        }
    }

    #[test]
    fn compatibility_cache_key_preserves_every_invalidation_dimension() {
        let original = key().normalize().unwrap();
        let mut variants = Vec::new();

        let mut changed = key();
        changed.workspace_identity.push_str("-other");
        variants.push(changed);

        let mut changed = key();
        changed.environment.build_directory = AuthoritativeValue::detected(
            "/workspace/other-build".into(),
            IdentityAuthority::InitializedEnvironment,
        );
        variants.push(changed);

        let mut changed = key();
        changed.environment.bitbake_version =
            AuthoritativeValue::detected("2.10.0".into(), IdentityAuthority::BitBakeVersionProbe);
        variants.push(changed);

        let mut changed = key();
        changed.environment.source_roots = AuthoritativeValue::detected(
            vec![SourceRootIdentity {
                kind: SourceRootKind::CoreBase,
                path: "/workspace/other-source".into(),
            }],
            IdentityAuthority::InitializedEnvironment,
        );
        variants.push(changed);

        let mut changed = key();
        changed.environment.available_tools = AuthoritativeValue::detected(
            vec![ToolIdentity {
                id: "bitbake".into(),
                executable: "/workspace/bitbake/bin/bitbake".into(),
                version: Some("2.8.2".into()),
            }],
            IdentityAuthority::ExecutableProbe,
        );
        variants.push(changed);

        let mut changed = key();
        changed.environment.layer_series = AuthoritativeValue::detected(
            vec![LayerSeriesIdentity {
                layer: "core".into(),
                root: "/workspace/meta".into(),
                compatible_series: vec!["scarthgap".into()],
            }],
            IdentityAuthority::ConfiguredLayerMetadata,
        );
        variants.push(changed);

        let mut changed = key();
        changed.initialized_environment_digest = digest('d');
        variants.push(changed);

        let mut changed = key();
        changed.layer_configuration_digest = digest('d');
        variants.push(changed);

        let mut changed = key();
        changed.build_configuration_digest = digest('d');
        variants.push(changed);

        let mut changed = key();
        changed.daemon_workspace_identity = "daemon-workspace-two".into();
        variants.push(changed);

        for variant in variants {
            assert_ne!(original, variant.normalize().unwrap());
        }
    }

    #[test]
    fn compatibility_cache_key_rejects_weak_digest_or_invalid_identity() {
        let mut invalid = key();
        invalid.layer_configuration_digest = "not-a-digest".into();
        assert_eq!(
            invalid.normalize(),
            Err(CapabilityCacheKeyError::InvalidField)
        );

        let mut invalid = key();
        invalid.workspace_identity = "workspace\nother".into();
        assert_eq!(
            invalid.normalize(),
            Err(CapabilityCacheKeyError::InvalidField)
        );
    }
}

#[cfg(test)]
mod environment_identity {
    use super::*;

    fn full_identity() -> YoctoEnvironmentIdentity {
        YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                PathBuf::from("/work/poky/build"),
                IdentityAuthority::InitializedEnvironment,
            ),
            source_roots: AuthoritativeValue::detected(
                vec![
                    SourceRootIdentity {
                        kind: SourceRootKind::Layer,
                        path: "/work/poky/meta-poky".into(),
                    },
                    SourceRootIdentity {
                        kind: SourceRootKind::CoreBase,
                        path: "/work/poky".into(),
                    },
                ],
                IdentityAuthority::ConfiguredLayerMetadata,
            ),
            bitbake_version: AuthoritativeValue::detected(
                "2.8.1".into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            oe_core: AuthoritativeValue::detected(
                ReleaseIdentity {
                    name: Some("scarthgap".into()),
                    version: Some("5.0.19".into()),
                },
                IdentityAuthority::ReleaseMetadata,
            ),
            poky: AuthoritativeValue::detected(
                ReleaseIdentity {
                    name: Some("scarthgap".into()),
                    version: Some("5.0.19".into()),
                },
                IdentityAuthority::ReleaseMetadata,
            ),
            distro: AuthoritativeValue::detected(
                DistroIdentity {
                    name: "poky".into(),
                    version: Some("5.0.19".into()),
                },
                IdentityAuthority::BitBakeDatastore,
            ),
            machine: AuthoritativeValue::detected(
                "qemux86-64".into(),
                IdentityAuthority::BitBakeDatastore,
            ),
            layer_series: AuthoritativeValue::detected(
                vec![
                    LayerSeriesIdentity {
                        layer: "meta-poky".into(),
                        root: "/work/poky/meta-poky".into(),
                        compatible_series: vec!["scarthgap".into()],
                    },
                    LayerSeriesIdentity {
                        layer: "core".into(),
                        root: "/work/poky/meta".into(),
                        compatible_series: vec!["scarthgap".into(), "nanbield".into()],
                    },
                ],
                IdentityAuthority::ConfiguredLayerMetadata,
            ),
            available_tools: AuthoritativeValue::detected(
                vec![ToolIdentity {
                    id: "bitbake".into(),
                    executable: "/work/poky/bitbake/bin/bitbake".into(),
                    version: Some("2.8.1".into()),
                }],
                IdentityAuthority::ExecutableProbe,
            ),
            backend: AuthoritativeValue::detected(
                BackendIdentity {
                    name: "tinfoil".into(),
                    version: Some("1".into()),
                },
                IdentityAuthority::BackendHandshake,
            ),
            protocol: AuthoritativeValue::detected(
                ProtocolIdentity {
                    name: "yoctui-daemon".into(),
                    version: "1.0".into(),
                },
                IdentityAuthority::ProtocolNegotiation,
            ),
        }
    }

    #[test]
    fn environment_identity_normalizes_deterministically_without_collapsing_mixed_series() {
        let normalized = full_identity().normalize().unwrap();
        let roots = normalized.source_roots.value().unwrap();
        assert_eq!(roots[0].kind, SourceRootKind::CoreBase);
        let layers = normalized.layer_series.value().unwrap();
        assert_eq!(layers[0].layer, "core");
        assert_eq!(
            layers[0].compatible_series,
            ["nanbield".to_string(), "scarthgap".to_string()]
        );
        assert_ne!(layers[0].compatible_series, layers[1].compatible_series);
    }

    #[test]
    fn environment_identity_preserves_unknown_for_every_partial_field() {
        let identity = YoctoEnvironmentIdentity {
            machine: AuthoritativeValue::detected(
                "qemuarm64".into(),
                IdentityAuthority::BitBakeDatastore,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
        .normalize()
        .unwrap();
        assert_eq!(
            identity.machine.value().map(String::as_str),
            Some("qemuarm64")
        );
        assert_eq!(identity.bitbake_version, AuthoritativeValue::Unknown);
        assert_eq!(identity.poky, AuthoritativeValue::Unknown);
        assert_eq!(identity.available_tools, AuthoritativeValue::Unknown);
    }

    #[test]
    fn environment_identity_rejects_weak_or_wrong_authority() {
        let mut identity = full_identity();
        identity.bitbake_version =
            AuthoritativeValue::detected("2.8.1".into(), IdentityAuthority::ReleaseMetadata);
        assert!(matches!(
            identity.normalize(),
            Err(EnvironmentIdentityError::InvalidAuthority {
                field: "bitbake_version",
                ..
            })
        ));
    }

    #[test]
    fn environment_identity_rejects_invalid_paths_text_and_empty_detected_collections() {
        let mut relative = full_identity();
        relative.build_directory = AuthoritativeValue::detected(
            "relative/build".into(),
            IdentityAuthority::InitializedEnvironment,
        );
        assert_eq!(
            relative.normalize(),
            Err(EnvironmentIdentityError::InvalidField("build_directory"))
        );

        let mut control = full_identity();
        control.machine =
            AuthoritativeValue::detected("qemu\narm".into(), IdentityAuthority::BitBakeDatastore);
        assert_eq!(
            control.normalize(),
            Err(EnvironmentIdentityError::InvalidField("machine"))
        );

        let mut empty = full_identity();
        empty.available_tools =
            AuthoritativeValue::detected(Vec::new(), IdentityAuthority::ExecutableProbe);
        assert_eq!(
            empty.normalize(),
            Err(EnvironmentIdentityError::InvalidField("available_tools"))
        );
    }

    #[test]
    fn environment_identity_rejects_conflicting_duplicate_tools_and_layers() {
        let mut tools = full_identity();
        let AuthoritativeValue::Detected { value, .. } = &mut tools.available_tools else {
            unreachable!();
        };
        value.push(ToolIdentity {
            id: "bitbake".into(),
            executable: "/other/bin/bitbake".into(),
            version: Some("2.8.1".into()),
        });
        assert!(matches!(
            tools.normalize(),
            Err(EnvironmentIdentityError::ConflictingDuplicate {
                field: "available_tools",
                ..
            })
        ));

        let mut layers = full_identity();
        let AuthoritativeValue::Detected { value, .. } = &mut layers.layer_series else {
            unreachable!();
        };
        value.push(LayerSeriesIdentity {
            layer: "core".into(),
            root: "/different/meta".into(),
            compatible_series: vec!["scarthgap".into()],
        });
        assert!(matches!(
            layers.normalize(),
            Err(EnvironmentIdentityError::ConflictingDuplicate {
                field: "layer_series",
                ..
            })
        ));
    }

    #[test]
    fn environment_identity_deduplicates_exact_authoritative_records() {
        let mut identity = full_identity();
        let AuthoritativeValue::Detected { value, .. } = &mut identity.source_roots else {
            unreachable!();
        };
        value.push(value[0].clone());
        let normalized = identity.normalize().unwrap();
        assert_eq!(normalized.source_roots.value().unwrap().len(), 2);
    }

    #[test]
    fn compatibility_future_unknown_identity_is_preserved_without_release_inference() {
        let identity = YoctoEnvironmentIdentity {
            bitbake_version: AuthoritativeValue::detected(
                "99.0.0".into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            oe_core: AuthoritativeValue::detected(
                ReleaseIdentity {
                    name: Some("future-series".into()),
                    version: None,
                },
                IdentityAuthority::ReleaseMetadata,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
        .normalize()
        .unwrap();
        assert_eq!(
            identity.bitbake_version.value().map(String::as_str),
            Some("99.0.0")
        );
        assert_eq!(
            identity.oe_core.value().unwrap().name.as_deref(),
            Some("future-series")
        );
        assert_eq!(identity.poky, AuthoritativeValue::Unknown);
    }
}
