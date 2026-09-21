fn definition_development(id: CapabilityId) -> Option<Definition> {
    use CapabilityId as Id;
    use CapabilityImplementationKind as Kind;
    use CapabilityToolId as Tool;

    Some(match id {
        Id::DevtoolModify => tool_command(
            "Devtool modify",
            Tool::Devtool,
            Some("modify"),
            &[],
            "devtool.modify.argv",
        ),
        // `devtool status --help` initializes workspace context on supported
        // Poky releases and can exceed the bounded read-only probe deadline.
        // The global help command is side-effect free and authoritatively
        // enumerates the status subcommand, so probe that command list while
        // retaining the exact `devtool status` execution requirement.
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
        Id::DevtoolEditRecipe => tool_command(
            "Devtool edit-recipe",
            Tool::Devtool,
            Some("edit-recipe"),
            &[],
            "devtool.edit_recipe.argv",
        ),
        Id::DevtoolUpdateRecipe => tool_command(
            "Devtool update-recipe",
            Tool::Devtool,
            Some("update-recipe"),
            &[],
            "devtool.update_recipe.argv",
        ),
        Id::DevtoolFinish => tool_command(
            "Devtool finish",
            Tool::Devtool,
            Some("finish"),
            &[],
            "devtool.finish.argv",
        ),
        Id::DevtoolDeployTarget => tool_command(
            "Devtool deploy-target",
            Tool::Devtool,
            Some("deploy-target"),
            &[],
            "devtool.deploy_target.argv",
        ),
        Id::DevtoolUndeployTarget => tool_command(
            "Devtool undeploy-target",
            Tool::Devtool,
            Some("undeploy-target"),
            &[],
            "devtool.undeploy_target.argv",
        ),
        Id::DevtoolReset => tool_command(
            "Devtool reset",
            Tool::Devtool,
            Some("reset"),
            &[],
            "devtool.reset.argv",
        ),
        Id::DevtoolUpgrade => tool_command(
            "Devtool upgrade",
            Tool::Devtool,
            Some("upgrade"),
            &[],
            "devtool.upgrade.argv",
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
        Id::BitBakeLayersCreateLayer => tool_command(
            "bitbake-layers create-layer",
            Tool::BitBakeLayers,
            Some("create-layer"),
            &[],
            "bitbake_layers.create_layer.argv",
        ),
        Id::BitBakeLayersCreateAndAddLayer => tool_command(
            "bitbake-layers create and add layer",
            Tool::BitBakeLayers,
            Some("create-layer"),
            &["--add-layer"],
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
