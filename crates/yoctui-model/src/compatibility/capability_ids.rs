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
    #[serde(rename = "bitbake_layers.show_recipes")]
    BitBakeLayersShowRecipes,
    #[serde(rename = "bitbake_layers.show_overlayed")]
    BitBakeLayersShowOverlayed,
    #[serde(rename = "bitbake_layers.create_layer")]
    BitBakeLayersCreateLayer,
    #[serde(rename = "bitbake_layers.create_and_add_layer")]
    BitBakeLayersCreateAndAddLayer,
    #[serde(rename = "bitbake_layers.add_layer")]
    BitBakeLayersAddLayer,
    #[serde(rename = "bitbake_layers.remove_layer")]
    BitBakeLayersRemoveLayer,
    #[serde(rename = "bitbake_config_build.list_fragments")]
    BitBakeConfigBuildListFragments,
    #[serde(rename = "bitbake_config_build.show_fragment")]
    BitBakeConfigBuildShowFragment,
    #[serde(rename = "bitbake_config_build.enable_fragment")]
    BitBakeConfigBuildEnableFragment,
    #[serde(rename = "bitbake_config_build.disable_fragment")]
    BitBakeConfigBuildDisableFragment,
    #[serde(rename = "bitbake_config_build.disable_all_fragments")]
    BitBakeConfigBuildDisableAllFragments,
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
    pub const ALL: [Self; 114] = [
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
        Self::BitBakeLayersShowRecipes,
        Self::BitBakeLayersShowOverlayed,
        Self::BitBakeLayersCreateLayer,
        Self::BitBakeLayersCreateAndAddLayer,
        Self::BitBakeLayersAddLayer,
        Self::BitBakeLayersRemoveLayer,
        Self::BitBakeConfigBuildListFragments,
        Self::BitBakeConfigBuildShowFragment,
        Self::BitBakeConfigBuildEnableFragment,
        Self::BitBakeConfigBuildDisableFragment,
        Self::BitBakeConfigBuildDisableAllFragments,
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
            Self::BitBakeLayersShowRecipes => "bitbake_layers.show_recipes",
            Self::BitBakeLayersShowOverlayed => "bitbake_layers.show_overlayed",
            Self::BitBakeLayersCreateLayer => "bitbake_layers.create_layer",
            Self::BitBakeLayersCreateAndAddLayer => "bitbake_layers.create_and_add_layer",
            Self::BitBakeLayersAddLayer => "bitbake_layers.add_layer",
            Self::BitBakeLayersRemoveLayer => "bitbake_layers.remove_layer",
            Self::BitBakeConfigBuildListFragments => "bitbake_config_build.list_fragments",
            Self::BitBakeConfigBuildShowFragment => "bitbake_config_build.show_fragment",
            Self::BitBakeConfigBuildEnableFragment => "bitbake_config_build.enable_fragment",
            Self::BitBakeConfigBuildDisableFragment => "bitbake_config_build.disable_fragment",
            Self::BitBakeConfigBuildDisableAllFragments => {
                "bitbake_config_build.disable_all_fragments"
            }
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
