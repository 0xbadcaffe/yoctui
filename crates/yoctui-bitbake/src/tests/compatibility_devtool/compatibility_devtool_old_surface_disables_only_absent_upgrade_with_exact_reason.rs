use super::*;

#[test]
fn compatibility_devtool_old_surface_disables_only_absent_upgrade_with_exact_reason() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/scripts/devtool");
    let authority = authority(
        build,
        executable,
        4,
        &[(CapabilityId::DevtoolModify, DEVTOOL_MODIFY_IMPLEMENTATION)],
        &[CapabilityId::DevtoolUpgrade],
    );
    let planner = DevtoolCommandPlanner::new(&authority, 4, build, executable).unwrap();
    planner
        .operation(&DevtoolOperation::Modify {
            recipe: "busybox".into(),
        })
        .unwrap();
    assert!(matches!(
        planner.operation(&DevtoolOperation::Upgrade {
            recipe: "busybox".into()
        }),
        Err(DevtoolCompatibilityError::Unavailable { reason, .. })
            if reason.contains("devtool.upgrade")
    ));
}
