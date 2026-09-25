fn definition_development(id: CapabilityId) -> Option<Definition> {
    use CapabilityId as Id;
    use CapabilityImplementationKind as Kind;
    use CapabilityToolId as Tool;

    Some(match id {
        Id::DevtoolAdd => tool_command(
            "Devtool add",
            Tool::Devtool,
            Some("add"),
            &[
                "--same-dir", "--no-same-dir", "--fetch", "--npm-dev", "--no-pypi",
                "--version", "--no-git", "--srcrev", "--autorev", "--srcbranch",
                "--binary", "--also-native", "--src-subdir", "--mirrors", "--provides",
            ],
            "devtool.add.argv",
        ),
        Id::DevtoolModify => tool_command(
            "Devtool modify",
            Tool::Devtool,
            Some("modify"),
            &[
                "--wildcard", "--extract", "--no-extract", "--same-dir", "--no-same-dir",
                "--branch", "--no-overrides", "--keep-temp", "--debug-build",
            ],
            "devtool.modify.argv",
        ),
        Id::DevtoolUpgrade => tool_command(
            "Devtool upgrade",
            Tool::Devtool,
            Some("upgrade"),
            &[
                "--stable", "--version", "--srcrev", "--srcbranch", "--branch",
                "--no-patch", "--no-overrides", "--same-dir", "--no-same-dir",
                "--keep-temp", "--keep-failure",
            ],
            "devtool.upgrade.argv",
        ),
        // `devtool status --help` initializes workspace context on supported
        // Poky releases and can exceed the bounded read-only probe deadline.
        // Global help authoritatively enumerates this option-free subcommand.
        Id::DevtoolStatus => (
            "Devtool status",
            vec![Tool::Devtool],
            vec![command(Tool::Devtool, Some("status"), &[])],
            Vec::new(),
            vec![
                executable(Tool::Devtool),
                CapabilityProbeSpec::CommandHelpText {
                    tool: Tool::Devtool,
                    needle: "status".into(),
                },
            ],
            implementation("devtool.status.argv", Kind::Command),
            None,
        ),
        Id::DevtoolLatestVersion => tool_command(
            "Devtool latest-version", Tool::Devtool, Some("latest-version"), &["--stable"],
            "devtool.latest_version.argv",
        ),
        Id::DevtoolCheckUpgradeStatus => tool_command(
            "Devtool check-upgrade-status", Tool::Devtool, Some("check-upgrade-status"),
            &["--stable", "--all"], "devtool.check_upgrade_status.argv",
        ),
        Id::DevtoolSearch => tool_command(
            "Devtool search", Tool::Devtool, Some("search"), &[], "devtool.search.argv",
        ),
        Id::DevtoolBuild => tool_command(
            "Devtool build", Tool::Devtool, Some("build"),
            &["--disable-parallel-make", "--clean"], "devtool.build.argv",
        ),
        Id::DevtoolIdeSdk => tool_command(
            "Devtool ide-sdk", Tool::Devtool, Some("ide-sdk"),
            &[
                "--mode", "--ide", "--target", "--gdbserver-port-start", "--no-host-check",
                "--ssh-exec", "--port", "--key", "--skip-bitbake", "--bitbake-k",
                "--no-strip", "--dry-run", "--show-status", "--no-preserve",
                "--no-check-space",
            ],
            "devtool.ide_sdk.argv",
        ),
        Id::DevtoolRename => tool_command(
            "Devtool rename", Tool::Devtool, Some("rename"),
            &["--version", "--no-srctree"], "devtool.rename.argv",
        ),
        Id::DevtoolEditRecipe => tool_command(
            "Devtool edit-recipe", Tool::Devtool, Some("edit-recipe"), &["--any-recipe"],
            "devtool.edit_recipe.argv",
        ),
        Id::DevtoolFindRecipe => tool_command(
            "Devtool find-recipe", Tool::Devtool, Some("find-recipe"), &["--any-recipe"],
            "devtool.find_recipe.argv",
        ),
        Id::DevtoolConfigureHelp => tool_command(
            "Devtool configure-help", Tool::Devtool, Some("configure-help"),
            &["--no-pager", "--no-header", "--arg"], "devtool.configure_help.argv",
        ),
        Id::DevtoolUpdateRecipe => tool_command(
            "Devtool update-recipe", Tool::Devtool, Some("update-recipe"),
            &[
                "--mode", "--initial-rev", "--append", "--wildcard-version", "--no-remove",
                "--no-overrides", "--dry-run", "--force-patch-refresh",
            ],
            "devtool.update_recipe.argv",
        ),
        Id::DevtoolReset => tool_command(
            "Devtool reset", Tool::Devtool, Some("reset"),
            &["--all", "--no-clean", "--remove-work"], "devtool.reset.argv",
        ),
        Id::DevtoolFinish => tool_command(
            "Devtool finish", Tool::Devtool, Some("finish"),
            &[
                "--mode", "--initial-rev", "--force", "--remove-work", "--no-clean",
                "--no-overrides", "--dry-run", "--force-patch-refresh",
            ],
            "devtool.finish.argv",
        ),
        Id::DevtoolDeployTarget => tool_command(
            "Devtool deploy-target", Tool::Devtool, Some("deploy-target"),
            &[
                "--no-host-check", "--show-status", "--dry-run", "--no-preserve",
                "--no-check-space", "--ssh-exec", "--port", "--key", "--strip", "--no-strip",
            ],
            "devtool.deploy_target.argv",
        ),
        Id::DevtoolUndeployTarget => tool_command(
            "Devtool undeploy-target", Tool::Devtool, Some("undeploy-target"),
            &[
                "--no-host-check", "--show-status", "--all", "--dry-run", "--ssh-exec",
                "--port", "--key",
            ],
            "devtool.undeploy_target.argv",
        ),
        Id::DevtoolBuildImage => tool_command(
            "Devtool build-image", Tool::Devtool, Some("build-image"), &["--add-packages"],
            "devtool.build_image.argv",
        ),
        Id::DevtoolCreateWorkspace => tool_command(
            "Devtool create-workspace", Tool::Devtool, Some("create-workspace"),
            &["--layerseries", "--create-only"], "devtool.create_workspace.argv",
        ),
        Id::DevtoolExport => tool_command(
            "Devtool export", Tool::Devtool, Some("export"),
            &["--file", "--overwrite", "--include", "--exclude"], "devtool.export.argv",
        ),
        Id::DevtoolExtract => tool_command(
            "Devtool extract", Tool::Devtool, Some("extract"),
            &["--branch", "--no-overrides", "--keep-temp"], "devtool.extract.argv",
        ),
        Id::DevtoolSync => tool_command(
            "Devtool sync", Tool::Devtool, Some("sync"), &["--branch", "--keep-temp"],
            "devtool.sync.argv",
        ),
        Id::DevtoolImport => tool_command(
            "Devtool import", Tool::Devtool, Some("import"), &["--overwrite"],
            "devtool.import.argv",
        ),
        Id::DevtoolMenuconfig => tool_command(
            "Devtool menuconfig", Tool::Devtool, Some("menuconfig"), &[],
            "devtool.menuconfig.argv",
        ),
        Id::RecipetoolCreate => tool_command(
            "Recipetool create",
            Tool::Recipetool,
            Some("create"),
            &[],
            "recipetool.create.argv",
        ),
        Id::RecipetoolCreateOutfile => tool_command(
            "Recipetool create with explicit output",
            Tool::Recipetool,
            Some("create"),
            &["--outfile"],
            "recipetool.create.outfile.argv",
        ),
        Id::RecipetoolAppendFile => tool_command(
            "Recipetool appendfile",
            Tool::Recipetool,
            Some("appendfile"),
            &[],
            "recipetool.appendfile.argv",
        ),
        Id::BitBakeLayersShowLayers => tool_command(
            "bitbake-layers show-layers",
            Tool::BitBakeLayers,
            Some("show-layers"),
            &[],
            "bitbake_layers.show_layers.argv",
        ),
        Id::BitBakeLayersShowRecipes => tool_command(
            "bitbake-layers show-recipes",
            Tool::BitBakeLayers,
            Some("show-recipes"),
            &[
                "-f",
                "-r",
                "-m",
                "-i",
                "-l",
                "-b",
                "--show-variants",
                "--mc",
            ],
            "bitbake_layers.show_recipes.argv",
        ),
        Id::BitBakeLayersShowOverlayed => tool_command(
            "bitbake-layers show-overlayed",
            Tool::BitBakeLayers,
            Some("show-overlayed"),
            &["-f", "-s", "--mc"],
            "bitbake_layers.show_overlayed.argv",
        ),
        Id::BitBakeLayersShowAppends => tool_command(
            "bitbake-layers show-appends",
            Tool::BitBakeLayers,
            Some("show-appends"),
            &["--mc"],
            "bitbake_layers.show_appends.argv",
        ),
        Id::BitBakeLayersShowCrossDepends => tool_command(
            "bitbake-layers show-cross-depends",
            Tool::BitBakeLayers,
            Some("show-cross-depends"),
            &["-f", "-i"],
            "bitbake_layers.show_cross_depends.argv",
        ),
        Id::BitBakeLayersCreateLayer => tool_command(
            "bitbake-layers create-layer",
            Tool::BitBakeLayers,
            Some("create-layer"),
            &[
                "--layerid",
                "--priority",
                "--example-recipe-name",
                "--example-recipe-version",
            ],
            "bitbake_layers.create_layer.argv",
        ),
        Id::BitBakeLayersCreateAndAddLayer => tool_command(
            "bitbake-layers create and add layer",
            Tool::BitBakeLayers,
            Some("create-layer"),
            &[
                "--add-layer",
                "--layerid",
                "--priority",
                "--example-recipe-name",
                "--example-recipe-version",
            ],
            "bitbake_layers.create_and_add_layer.argv",
        ),
        Id::BitBakeLayersAddLayer => tool_command(
            "bitbake-layers add-layer",
            Tool::BitBakeLayers,
            Some("add-layer"),
            &[],
            "bitbake_layers.add_layer.argv",
        ),
        Id::BitBakeLayersRemoveLayer => tool_command(
            "bitbake-layers remove-layer",
            Tool::BitBakeLayers,
            Some("remove-layer"),
            &[],
            "bitbake_layers.remove_layer.argv",
        ),
        Id::BitBakeLayersFlatten => tool_command(
            "bitbake-layers flatten",
            Tool::BitBakeLayers,
            Some("flatten"),
            &[],
            "bitbake_layers.flatten.argv",
        ),
        Id::BitBakeLayersLayerIndexFetch => tool_command(
            "bitbake-layers layerindex-fetch",
            Tool::BitBakeLayers,
            Some("layerindex-fetch"),
            &["-n", "-b", "-s", "-i", "-f"],
            "bitbake_layers.layerindex_fetch.argv",
        ),
        Id::BitBakeLayersLayerIndexShowDepends => tool_command(
            "bitbake-layers layerindex-show-depends",
            Tool::BitBakeLayers,
            Some("layerindex-show-depends"),
            &["-b"],
            "bitbake_layers.layerindex_show_depends.argv",
        ),
        Id::BitBakeLayersShowMachines => tool_command(
            "bitbake-layers show-machines",
            Tool::BitBakeLayers,
            Some("show-machines"),
            &["-b", "-l"],
            "bitbake_layers.show_machines.argv",
        ),
        Id::BitBakeLayersSaveBuildConf => tool_command(
            "bitbake-layers save-build-conf",
            Tool::BitBakeLayers,
            Some("save-build-conf"),
            &[],
            "bitbake_layers.save_build_conf.argv",
        ),
        Id::BitBakeLayersCreateLayersSetup => tool_command(
            "bitbake-layers create-layers-setup",
            Tool::BitBakeLayers,
            Some("create-layers-setup"),
            &[
                "--output-prefix",
                "--writer",
                "--json-only",
                "--update",
                "--use-custom-reference",
            ],
            "bitbake_layers.create_layers_setup.argv",
        ),
        Id::BitBakeConfigBuildListFragments => tool_command(
            "bitbake-config-build list-fragments",
            Tool::BitBakeConfigBuild,
            Some("list-fragments"),
            &[],
            "bitbake_config_build.list_fragments.argv",
        ),
        Id::BitBakeConfigBuildShowFragment => tool_command(
            "bitbake-config-build show-fragment",
            Tool::BitBakeConfigBuild,
            Some("show-fragment"),
            &[],
            "bitbake_config_build.show_fragment.argv",
        ),
        Id::BitBakeConfigBuildEnableFragment => tool_command(
            "bitbake-config-build enable-fragment",
            Tool::BitBakeConfigBuild,
            Some("enable-fragment"),
            &[],
            "bitbake_config_build.enable_fragment.argv",
        ),
        Id::BitBakeConfigBuildDisableFragment => tool_command(
            "bitbake-config-build disable-fragment",
            Tool::BitBakeConfigBuild,
            Some("disable-fragment"),
            &[],
            "bitbake_config_build.disable_fragment.argv",
        ),
        Id::BitBakeConfigBuildDisableAllFragments => tool_command(
            "bitbake-config-build disable-all-fragments",
            Tool::BitBakeConfigBuild,
            Some("disable-all-fragments"),
            &[],
            "bitbake_config_build.disable_all_fragments.argv",
        ),
        Id::PkgDataLookupPackage => tool_command(
            "package-data package lookup",
            Tool::OePkgdataUtil,
            Some("lookup-pkg"),
            &[],
            "pkgdata.lookup_pkg.argv",
        ),
        Id::PkgDataFindPath => tool_command(
            "package-data path lookup",
            Tool::OePkgdataUtil,
            Some("find-path"),
            &[],
            "pkgdata.find_path.argv",
        ),
        Id::PkgDataGenerated => (
            "generated package data",
            Vec::new(),
            Vec::new(),
            vec![MetadataRequirement::Artifact {
                kind: "pkgdata".into(),
            }],
            vec![CapabilityProbeSpec::Artifact {
                kind: "pkgdata".into(),
            }],
            implementation("pkgdata.generated", Kind::ProcessAdapter),
            None,
        ),
        Id::PkgDataListPackages => tool_command(
            "package-data package inventory",
            Tool::OePkgdataUtil,
            Some("list-pkgs"),
            &["-r"],
            "pkgdata.list_packages.argv",
        ),
        Id::PkgDataPackageInfo => tool_command(
            "package-data package information",
            Tool::OePkgdataUtil,
            Some("package-info"),
            &["-e"],
            "pkgdata.package_info.argv",
        ),
        Id::PkgDataListPackageFiles => tool_command(
            "package-data file inventory",
            Tool::OePkgdataUtil,
            Some("list-pkg-files"),
            &["-r"],
            "pkgdata.list_package_files.argv",
        ),
        Id::PkgDataReadValue => tool_command(
            "package-data value query",
            Tool::OePkgdataUtil,
            Some("read-value"),
            &["-n"],
            "pkgdata.read_value.argv",
        ),
        _ => return None,
    })
}
