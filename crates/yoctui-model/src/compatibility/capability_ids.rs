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
    #[serde(rename = "devtool.add")]
    DevtoolAdd,
    #[serde(rename = "devtool.latest_version")]
    DevtoolLatestVersion,
    #[serde(rename = "devtool.check_upgrade_status")]
    DevtoolCheckUpgradeStatus,
    #[serde(rename = "devtool.search")]
    DevtoolSearch,
    #[serde(rename = "devtool.build")]
    DevtoolBuild,
    #[serde(rename = "devtool.ide_sdk")]
    DevtoolIdeSdk,
    #[serde(rename = "devtool.rename")]
    DevtoolRename,
    #[serde(rename = "devtool.find_recipe")]
    DevtoolFindRecipe,
    #[serde(rename = "devtool.configure_help")]
    DevtoolConfigureHelp,
    #[serde(rename = "devtool.build_image")]
    DevtoolBuildImage,
    #[serde(rename = "devtool.create_workspace")]
    DevtoolCreateWorkspace,
    #[serde(rename = "devtool.export")]
    DevtoolExport,
    #[serde(rename = "devtool.extract")]
    DevtoolExtract,
    #[serde(rename = "devtool.sync")]
    DevtoolSync,
    #[serde(rename = "devtool.import")]
    DevtoolImport,
    #[serde(rename = "devtool.menuconfig")]
    DevtoolMenuconfig,
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
    #[serde(rename = "bitbake_layers.show_appends")]
    BitBakeLayersShowAppends,
    #[serde(rename = "bitbake_layers.show_cross_depends")]
    BitBakeLayersShowCrossDepends,
    #[serde(rename = "bitbake_layers.create_layer")]
    BitBakeLayersCreateLayer,
    #[serde(rename = "bitbake_layers.create_and_add_layer")]
    BitBakeLayersCreateAndAddLayer,
    #[serde(rename = "bitbake_layers.add_layer")]
    BitBakeLayersAddLayer,
    #[serde(rename = "bitbake_layers.remove_layer")]
    BitBakeLayersRemoveLayer,
    #[serde(rename = "bitbake_layers.flatten")]
    BitBakeLayersFlatten,
    #[serde(rename = "bitbake_layers.layerindex_fetch")]
    BitBakeLayersLayerIndexFetch,
    #[serde(rename = "bitbake_layers.layerindex_show_depends")]
    BitBakeLayersLayerIndexShowDepends,
    #[serde(rename = "bitbake_layers.show_machines")]
    BitBakeLayersShowMachines,
    #[serde(rename = "bitbake_layers.save_build_conf")]
    BitBakeLayersSaveBuildConf,
    #[serde(rename = "bitbake_layers.create_layers_setup")]
    BitBakeLayersCreateLayersSetup,
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
    pub const ALL: [Self; 138] = [
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
        Self::DevtoolAdd,
        Self::DevtoolLatestVersion,
        Self::DevtoolCheckUpgradeStatus,
        Self::DevtoolSearch,
        Self::DevtoolBuild,
        Self::DevtoolIdeSdk,
        Self::DevtoolRename,
        Self::DevtoolFindRecipe,
        Self::DevtoolConfigureHelp,
        Self::DevtoolBuildImage,
        Self::DevtoolCreateWorkspace,
        Self::DevtoolExport,
        Self::DevtoolExtract,
        Self::DevtoolSync,
        Self::DevtoolImport,
        Self::DevtoolMenuconfig,
        Self::RecipetoolCreate,
        Self::RecipetoolCreateOutfile,
        Self::RecipetoolAppendFile,
        Self::BitBakeLayersShowLayers,
        Self::BitBakeLayersShowRecipes,
        Self::BitBakeLayersShowOverlayed,
        Self::BitBakeLayersShowAppends,
        Self::BitBakeLayersShowCrossDepends,
        Self::BitBakeLayersCreateLayer,
        Self::BitBakeLayersCreateAndAddLayer,
        Self::BitBakeLayersAddLayer,
        Self::BitBakeLayersRemoveLayer,
        Self::BitBakeLayersFlatten,
        Self::BitBakeLayersLayerIndexFetch,
        Self::BitBakeLayersLayerIndexShowDepends,
        Self::BitBakeLayersShowMachines,
        Self::BitBakeLayersSaveBuildConf,
        Self::BitBakeLayersCreateLayersSetup,
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
        capability_id_name(self)
    }

    pub fn from_stable_name(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.as_str() == value)
    }
}

include!("capability_names.rs");

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
