/// Closed contextual action inventory. It represents the useful operations
/// users can launch from each workspace; renderers consume this data and never
/// embed capability IDs or release policy.
pub(crate) fn compatibility_ui_workspace_action_seeds(
    destination: crate::WorkspaceDestination,
) -> Vec<CompatibilityUiWorkspaceActionDefinition> {
    use crate::WorkspaceDestination as Destination;
    use CapabilityId as Id;
    use CompatibilityUiWorkspaceActionDefinition as Action;
    match destination {
        Destination::Dashboard => vec![
            Action::capability("dashboard.build", "Build image", "Alt+b", Id::BitBakeBuild),
            Action::capability(
                "dashboard.cancel",
                "Cancel active build",
                "c",
                Id::BitBakeCancellation,
            ),
            Action::local("dashboard.tasks", "Monitor active tasks", "F2"),
            Action::local("dashboard.logs", "Open retained logs", "l"),
            Action::local("dashboard.errors", "Review failures", "e"),
            Action::local("dashboard.history", "Inspect recent work", "F3"),
            Action::local("dashboard.artifacts", "Inspect artifacts", "F8"),
            Action::local("dashboard.environment", "Configure build environment", "Alt+e"),
            Action::local("dashboard.maintenance", "Sstate readiness", "Alt+m"),
            Action::local("dashboard.favorites", "Browse favorite commands", "f"),
            Action::local("dashboard.terminals", "Open terminal sessions", "t"),
        ],
        Destination::Hardware => vec![
            Action::local("hardware.add", "Add document", "a"),
            Action::local("hardware.view", "View selected document", "Enter"),
            Action::local("hardware.remove", "Remove selected document", "d"),
            Action::local("hardware.reload", "Reload selected document", "r"),
        ],
        Destination::Recipes => vec![
            Action::capability(
                "recipes.metadata",
                "Refresh metadata",
                "r",
                Id::BitBakeRecipeMetadata,
            ),
            Action::capability(
                "recipes.dependencies",
                "Dependencies",
                "Alt+a",
                Id::BitBakeRecipeDependencies,
            ),
            Action::capability(
                "recipes.build",
                "Build selected recipe",
                "b",
                Id::BitBakeBuild,
            ),
            Action::all(
                "recipes.force_task",
                "Force selected task",
                "f",
                &[Id::BitBakeBuild, Id::BitBakeForceTask],
            ),
            Action::capability(
                "recipes.signatures",
                "Inspect signatures",
                "z",
                Id::BitBakeDumpSig,
            ),
            Action::all(
                "recipes.cve",
                "Run CVE check",
                "Alt+v",
                &[Id::BitBakeBuild, Id::CveCheck],
            ),
            Action::all(
                "recipes.spdx",
                "Create SPDX",
                "Alt+x",
                &[Id::BitBakeBuild, Id::SpdxCreate],
            ),
            Action::capability(
                "recipes.devtool_modify",
                "Devtool modify",
                "d/Alt+w",
                Id::DevtoolModify,
            ),
            Action::local(
                "recipes.devtool_gitui",
                "Open Devtool workspace GitUI",
                "Alt+g",
            ),
            Action::capability(
                "recipes.devtool_update",
                "Devtool update-recipe",
                "u",
                Id::DevtoolUpdateRecipe,
            ),
            Action::capability(
                "recipes.devtool_finish",
                "Devtool finish",
                "Alt+f",
                Id::DevtoolFinish,
            ),
            Action::capability(
                "recipes.devtool_deploy",
                "Devtool deploy-target",
                "Alt+p",
                Id::DevtoolDeployTarget,
            ),
            Action::capability(
                "recipes.devtool_reset",
                "Devtool reset",
                "Alt+d",
                Id::DevtoolReset,
            ),
            Action::local("recipes.open", "Open provider/log/source", "Enter/o/e"),
        ],
        Destination::Layers => vec![
            Action::alternatives(
                "layers.inventory",
                "Refresh layer inventory",
                "r",
                &[Id::BitBakeLayerInventory, Id::BitBakeLayersShowLayers],
            ),
            Action::capability(
                "layers.relationships",
                "Layer relationships",
                "Alt+r",
                Id::BitBakeLayerRelationships,
            ),
            Action::capability(
                "layers.create",
                "Create layer",
                "c",
                Id::BitBakeLayersCreateLayer,
            ),
            Action::capability("layers.add", "Add layer", "a", Id::BitBakeLayersAddLayer),
            Action::capability(
                "layers.remove",
                "Remove layer",
                "x",
                Id::BitBakeLayersRemoveLayer,
            ),
            Action::local("layers.open", "Browse/edit configured layer", "Enter/e/o"),
        ],
        Destination::Configuration => vec![
            Action::capability(
                "configuration.getvar",
                "Refresh effective variables",
                "r",
                Id::BitBakeGetVar,
            ),
            Action::local(
                "configuration.inspect",
                "Inspect/copy/source",
                "Enter/Alt+c/Alt+u/o",
            ),
            Action::local("configuration.edit", "Edit local assignment", "Alt+e/x"),
        ],
        Destination::Tasks => vec![
            Action::capability(
                "tasks.inventory",
                "Inspect task inventory",
                "F2",
                Id::BitBakeTaskList,
            ),
            Action::capability("tasks.build", "Build options", "Alt+b", Id::BitBakeBuild),
            Action::capability(
                "tasks.cancel",
                "Cancel active build",
                "c",
                Id::BitBakeCancellation,
            ),
            Action::local("tasks.logs", "Open Logs", "l"),
            Action::local("tasks.history", "Build History", "h"),
        ],
        Destination::BuildHistory => vec![Action::local(
            "build_history.inspect",
            "Inspect retained build record",
            "Enter",
        )],
        Destination::Logs => vec![Action::local(
            "logs.inspect",
            "Filter/bookmark/copy/export/open retained logs",
            "/m/[/]/Alt+c/Alt+e/o",
        )],
        Destination::Errors => vec![Action::local(
            "errors.inspect",
            "Inspect retained diagnostic source",
            "Enter/o",
        )],
        Destination::Dependencies => vec![
            Action::alternatives(
                "dependencies.refresh",
                "Choose or refresh dependency root",
                "r",
                &[Id::BitBakeRecipeDependencies, Id::BitBakeDependencyGraph],
            ),
            Action::local("dependencies.open", "Open provider/task log", "Enter/o/Alt+l"),
        ],
        Destination::Signatures => vec![
            Action::capability(
                "signatures.dump",
                "Dump task signature",
                "r",
                Id::BitBakeDumpSig,
            ),
            Action::capability(
                "signatures.compare",
                "Compare signatures",
                "c",
                Id::BitBakeDiffSigs,
            ),
            Action::local("signatures.open", "Open provider", "e"),
        ],
        Destination::Packages => vec![
            Action::all(
                "packages.inventory",
                "Refresh package inventory",
                "Alt+r",
                &[Id::PkgDataGenerated, Id::PkgDataListPackages],
            ),
            Action::all(
                "packages.detail",
                "Load package details",
                "Enter",
                &[
                    Id::PkgDataGenerated,
                    Id::PkgDataPackageInfo,
                    Id::PkgDataListPackageFiles,
                    Id::PkgDataReadValue,
                ],
            ),
            Action::local(
                "packages.navigate",
                "Navigate/open package evidence",
                "[/]/d/u/o/e",
            ),
            Action::local("packages.cancel", "Cancel owned package scan", "c"),
        ],
        Destination::Images => vec![
            Action::capability(
                "images.build",
                "Build selected image",
                "b",
                Id::BitBakeBuild,
            ),
            Action::capability("images.qemu", "Launch QEMU", "Alt+q", Id::RunQemu),
            Action::local("images.console", "Open image console", "Alt+t"),
            Action::capability("images.wic", "Create Wic image", "Alt+w", Id::WicCreate),
            Action::local("images.device_write", "Write selected local device", "Alt+d"),
            Action::local("images.artifacts", "Scan/open deployed artifacts", "Alt+r/o/Alt+o"),
            Action::local(
                "images.rootfs",
                "Inspect selected rootfs composition",
                "Enter/p/Tab",
            ),
            Action::local("images.cancel", "Cancel owned image operation", "x/c"),
        ],
        Destination::Kernel => kernel_workspace_action_seeds(),
        Destination::Firmware => firmware_workspace_action_seeds(),
        Destination::Sdk => vec![
            Action::all(
                "sdk.standard",
                "Populate standard SDK",
                "s",
                &[Id::BitBakeBuild, Id::SdkPopulate],
            ),
            Action::all(
                "sdk.extensible",
                "Populate extensible SDK",
                "Alt+e",
                &[Id::BitBakeBuild, Id::SdkExtensible],
            ),
            Action::all(
                "sdk.testsdk",
                "Run testsdk",
                "t",
                &[Id::BitBakeBuild, Id::TestSdk],
            ),
            Action::all(
                "sdk.testsdkext",
                "Run testsdkext",
                "Alt+t",
                &[Id::BitBakeBuild, Id::TestSdkExtensible],
            ),
            Action::capability("sdk.publish", "Publish SDK", "Alt+p", Id::SdkPublish),
            Action::capability("sdk.native", "Run native SDK tool", "n", Id::SdkNativeTools),
            Action::local("sdk.artifacts", "Scan/open SDK artifacts", "Alt+r/o"),
            Action::local("sdk.cancel", "Cancel owned SDK operation", "c"),
        ],
        Destination::Testing => vec![
            Action::capability(
                "testing.oe_selftest",
                "Run oe-selftest",
                "r",
                Id::OeSelftest,
            ),
            Action::capability(
                "testing.bitbake_selftest",
                "Run BitBake selftest",
                "r",
                Id::BitBakeSelftest,
            ),
            Action::all(
                "testing.testimage",
                "Run testimage",
                "r",
                &[Id::BitBakeBuild, Id::TestImage],
            ),
            Action::all(
                "testing.testsdk",
                "Run testsdk",
                "r",
                &[Id::BitBakeBuild, Id::TestSdk],
            ),
            Action::all(
                "testing.testsdkext",
                "Run testsdkext",
                "r",
                &[Id::BitBakeBuild, Id::TestSdkExtensible],
            ),
            Action::all(
                "testing.ptest",
                "Run ptest",
                "r",
                &[Id::BitBakeBuild, Id::Ptest],
            ),
            Action::capability("testing.compare", "Compare results", "c", Id::ResultTool),
            Action::local("testing.import", "Import/open/export results", "Alt+i/o/Alt+j"),
            Action::local("testing.cancel", "Cancel owned test operation", "x"),
        ],
        Destination::Security => vec![
            Action::all(
                "security.cve",
                "Run CVE check",
                "Alt+v",
                &[Id::BitBakeBuild, Id::CveCheck],
            ),
            Action::all(
                "security.spdx",
                "Create SPDX/SBOM",
                "Alt+x",
                &[Id::BitBakeBuild, Id::SpdxCreate],
            ),
            Action::all(
                "security.package_map",
                "Map package data",
                "Alt+m",
                &[Id::PkgDataGenerated, Id::PkgDataLookupPackage],
            ),
            Action::local(
                "security.reports",
                "Import/open security evidence",
                "Alt+i/Alt+r/o/e/v",
            ),
            Action::local("security.cancel", "Cancel owned security operation", "c"),
        ],
        Destination::Qa => vec![
            Action::all(
                "qa.recipe",
                "Run recipe QA task",
                "r",
                &[Id::BitBakeBuild, Id::QaTask],
            ),
            Action::capability(
                "qa.layer",
                "Run layer compatibility check",
                "r",
                Id::YoctoCheckLayer,
            ),
            Action::local("qa.reports", "Import/open QA evidence", "Alt+i/Alt+r/o/e/l"),
            Action::local("qa.cancel", "Cancel owned QA operation", "c"),
        ],
        Destination::RawMode => vec![Action::local(
            "raw.inspect",
            "Inspect Raw command catalog",
            "Enter",
        )],
        Destination::Devtool => vec![
            Action::capability(
                "devtool.status",
                "Refresh Devtool status",
                "r",
                Id::DevtoolStatus,
            ),
            Action::capability("devtool.modify", "Modify recipe", "d/e/Alt+w", Id::DevtoolModify),
            Action::local(
                "devtool.gitui",
                "Open workspace GitUI",
                "Alt+g",
            ),
            Action::local(
                "devtool.shell",
                "Open workspace shell",
                "F12 Actions",
            ),
            Action::capability(
                "devtool.build",
                "Compile workspace recipe",
                "F12 Actions",
                Id::BitBakeBuild,
            ),
            Action::capability("devtool.edit", "Edit recipe file", "Alt+e", Id::DevtoolEditRecipe),
            Action::capability(
                "devtool.update",
                "Create/update patches in layer",
                "u",
                Id::DevtoolUpdateRecipe,
            ),
            Action::capability("devtool.finish", "Finish recipe", "Alt+f", Id::DevtoolFinish),
            Action::capability(
                "devtool.deploy",
                "Deploy build with SSH/SCP",
                "Alt+p",
                Id::DevtoolDeployTarget,
            ),
            Action::capability(
                "devtool.undeploy",
                "Undeploy target",
                "Alt+p",
                Id::DevtoolUndeployTarget,
            ),
            Action::capability("devtool.reset", "Reset recipe", "Alt+d", Id::DevtoolReset),
            Action::capability("devtool.upgrade", "Upgrade recipe", "Alt+u", Id::DevtoolUpgrade),
        ],
        Destination::QemuWic => vec![
            Action::capability("qemu_wic.qemu", "Launch QEMU", "Alt+q", Id::RunQemu),
            Action::capability("qemu_wic.wic", "Create Wic image", "Alt+w", Id::WicCreate),
            Action::local("qemu_wic.write", "Write local block device", "Alt+d"),
            Action::local("qemu_wic.cancel", "Cancel owned runtime", "x"),
        ],
        Destination::Maintenance => vec![
            Action::capability(
                "maintenance.readiness",
                "Check sstate readiness",
                "c",
                Id::SstateReadiness,
            ),
            Action::capability(
                "maintenance.cleanup",
                "Clean shared state",
                "d",
                Id::SstateCleanup,
            ),
            Action::capability(
                "maintenance.prserv",
                "Manage PR service",
                "e/m",
                Id::PrservManagement,
            ),
            Action::capability(
                "maintenance.locked",
                "Generate locked signatures",
                "l",
                Id::LockedSignatures,
            ),
            Action::capability(
                "maintenance.history",
                "Compare build history",
                "h",
                Id::BuildHistoryCompare,
            ),
            Action::capability(
                "maintenance.archive",
                "Archive repository",
                "a",
                Id::GitArchive,
            ),
            Action::local(
                "maintenance.cancel",
                "Cancel owned maintenance operation",
                "x",
            ),
            Action::local("maintenance.evidence", "Open retained evidence", "o"),
        ],
        Destination::TerminalSessions => vec![
            Action::local("terminal.shell", "Open build shell", "n"),
            Action::capability("terminal.devshell", "Open devshell", "s", Id::DevShell),
            Action::capability(
                "terminal.menuconfig",
                "Open menuconfig",
                "m",
                Id::MenuConfig,
            ),
            Action::local("terminal.control", "Take writer control", "o"),
            Action::local("terminal.cancel", "Release writer control", "c"),
        ],
        Destination::ProjectProfiles
        | Destination::BuildEnvironment
        | Destination::Compatibility
        | Destination::Daemons
        | Destination::Settings
        | Destination::Help => Vec::new(),
    }
}
