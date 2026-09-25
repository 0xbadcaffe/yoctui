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

#[test]
fn devtool_patch_planner_preserves_the_native_configured_layer_argument() {
    use std::os::unix::ffi::OsStringExt;

    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/scripts/devtool");
    let authority = authority(
        build,
        executable,
        9,
        &[(
            CapabilityId::DevtoolUpdateRecipe,
            DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION,
        )],
        &[],
    );
    let destination = PathBuf::from(std::ffi::OsString::from_vec(
        b"/layers/meta-custom-\xff".to_vec(),
    ));
    let operation = DevtoolOperation::UpdateRecipePatch {
        recipe: "busybox".into(),
        destination: destination.clone(),
    };

    let command = DevtoolCommandPlanner::new(&authority, 9, build, executable)
        .unwrap()
        .operation(&operation)
        .unwrap();
    assert_eq!(command.arguments()[0], "update-recipe");
    assert_eq!(command.arguments()[1], "--mode");
    assert_eq!(command.arguments()[2], "patch");
    assert_eq!(command.arguments()[3], "--append");
    assert_eq!(command.arguments()[4], destination.as_os_str());
    assert_eq!(command.arguments()[5], "busybox");
}
