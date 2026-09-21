use super::*;

#[test]
fn compatibility_devtool_rejects_stale_environment_executable_and_cross_subcommand_authority() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/scripts/devtool");
    let authority = authority(
        build,
        executable,
        5,
        &[(CapabilityId::DevtoolModify, DEVTOOL_MODIFY_IMPLEMENTATION)],
        &[],
    );
    assert!(matches!(
        DevtoolCommandPlanner::new(&authority, 4, build, executable),
        Err(DevtoolCompatibilityError::StaleGeneration { .. })
    ));
    assert!(matches!(
        DevtoolCommandPlanner::new(&authority, 5, Path::new("/other"), executable),
        Err(DevtoolCompatibilityError::EnvironmentMismatch)
    ));
    assert!(matches!(
        DevtoolCommandPlanner::new(&authority, 5, build, Path::new("/usr/bin/devtool")),
        Err(DevtoolCompatibilityError::ExecutableMismatch)
    ));
    let planner = DevtoolCommandPlanner::new(&authority, 5, build, executable).unwrap();
    assert!(matches!(
        planner.operation(&DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "/layers/meta".into()
        }),
        Err(DevtoolCompatibilityError::CapabilityMissing {
            capability: CapabilityId::DevtoolFinish
        })
    ));
}
