use super::*;

#[test]
fn compatibility_command_shared_fixtures_gate_devtool_upgrade_before_argv() {
    let authority = |role, generation| {
        release_capability_fixtures()
            .into_iter()
            .find(|fixture| fixture.role == role)
            .unwrap()
            .command_authority(generation)
    };
    let upgrade = DevtoolOperation::Upgrade {
        recipe: "busybox".into(),
    };

    let old = authority(CompatibilityFixtureRole::OldestPolicyCandidate, 33);
    let old_build = old.snapshot.environment.build_directory.value().unwrap();
    let old_tool = old
        .snapshot
        .environment
        .available_tools
        .value()
        .unwrap()
        .iter()
        .find(|tool| tool.id == "devtool")
        .unwrap()
        .executable
        .as_path();
    let old_planner = DevtoolCommandPlanner::new(&old, 33, old_build, old_tool).unwrap();
    assert!(matches!(
        old_planner.operation(&upgrade),
        Err(DevtoolCompatibilityError::Unavailable {
            capability: CapabilityId::DevtoolUpgrade,
            ..
        })
    ));

    let modern = authority(CompatibilityFixtureRole::LatestSupportCandidate, 34);
    let modern_build = modern.snapshot.environment.build_directory.value().unwrap();
    let modern_tool = modern
        .snapshot
        .environment
        .available_tools
        .value()
        .unwrap()
        .iter()
        .find(|tool| tool.id == "devtool")
        .unwrap()
        .executable
        .as_path();
    let command = DevtoolCommandPlanner::new(&modern, 34, modern_build, modern_tool)
        .unwrap()
        .operation(&upgrade)
        .unwrap();
    assert_eq!(command.arguments(), ["upgrade", "busybox"]);
}
