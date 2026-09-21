use super::*;

#[test]
fn compatibility_devtool_generates_exact_argv_for_each_independently_probed_subcommand() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/scripts/devtool");
    let available = [
        (CapabilityId::DevtoolStatus, DEVTOOL_STATUS_IMPLEMENTATION),
        (
            CapabilityId::DevtoolEditRecipe,
            DEVTOOL_EDIT_RECIPE_IMPLEMENTATION,
        ),
        (CapabilityId::DevtoolModify, DEVTOOL_MODIFY_IMPLEMENTATION),
        (
            CapabilityId::DevtoolUpdateRecipe,
            DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION,
        ),
        (CapabilityId::DevtoolFinish, DEVTOOL_FINISH_IMPLEMENTATION),
        (
            CapabilityId::DevtoolDeployTarget,
            DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION,
        ),
        (
            CapabilityId::DevtoolUndeployTarget,
            DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION,
        ),
        (CapabilityId::DevtoolReset, DEVTOOL_RESET_IMPLEMENTATION),
        (CapabilityId::DevtoolUpgrade, DEVTOOL_UPGRADE_IMPLEMENTATION),
    ];
    let authority = authority(build, executable, 3, &available, &[]);
    let planner = DevtoolCommandPlanner::new(&authority, 3, build, executable).unwrap();
    assert_eq!(planner.status().unwrap().arguments(), ["status"]);
    assert_eq!(
        planner.edit_recipe("busybox").unwrap().arguments(),
        ["edit-recipe", "busybox"]
    );
    for (operation, capability, expected) in all_operations() {
        let command = planner.operation(&operation).unwrap();
        assert_eq!(command.capability(), capability);
        assert_eq!(command.capability_generation(), 3);
        assert_eq!(command.arguments(), expected);
        assert_eq!(command.executable(), executable);
    }
}
