use super::*;

#[test]
fn compatibility_command_shared_fixtures_gate_recipetool_outfile_before_argv() {
    let authority = |role, generation| {
        release_capability_fixtures()
            .into_iter()
            .find(|fixture| fixture.role == role)
            .unwrap()
            .command_authority(generation)
    };

    let old = authority(CompatibilityFixtureRole::OldestPolicyCandidate, 35);
    let old_build = old.snapshot.environment.build_directory.value().unwrap();
    let old_tool = old
        .snapshot
        .environment
        .available_tools
        .value()
        .unwrap()
        .iter()
        .find(|tool| tool.id == "recipetool")
        .unwrap()
        .executable
        .as_path();
    let old_planner = RecipetoolCommandPlanner::new(&old, 35, old_build, old_tool).unwrap();
    assert!(matches!(
        old_planner.operation(&create()),
        Err(RecipetoolCompatibilityError::Unavailable {
            capability: CapabilityId::RecipetoolCreateOutfile,
            ..
        })
    ));
    assert_eq!(
        old_planner.operation(&appendfile()).unwrap().arguments(),
        ["appendfile", "/layers/meta-demo", "/etc/motd", "/work/motd"]
    );

    let modern = authority(CompatibilityFixtureRole::LatestSupportCandidate, 36);
    let modern_build = modern.snapshot.environment.build_directory.value().unwrap();
    let modern_tool = modern
        .snapshot
        .environment
        .available_tools
        .value()
        .unwrap()
        .iter()
        .find(|tool| tool.id == "recipetool")
        .unwrap()
        .executable
        .as_path();
    assert_eq!(
        RecipetoolCommandPlanner::new(&modern, 36, modern_build, modern_tool)
            .unwrap()
            .operation(&create())
            .unwrap()
            .arguments(),
        [
            "create",
            "--outfile",
            "/layers/meta-demo/recipes-demo/demo.bb",
            "https://example.invalid/demo.tar.gz",
        ]
    );
}
