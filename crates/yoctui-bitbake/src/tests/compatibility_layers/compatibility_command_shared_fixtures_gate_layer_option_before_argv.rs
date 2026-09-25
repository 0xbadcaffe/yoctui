use super::*;

#[test]
fn compatibility_command_shared_fixtures_gate_layer_option_before_argv() {
    let authority = |role, generation| {
        release_capability_fixtures()
            .into_iter()
            .find(|fixture| fixture.role == role)
            .unwrap()
            .command_authority(generation)
    };
    let operation = BitBakeLayersOperation::CreateLayer {
        directory: "/layers/meta-demo".into(),
        add: true,
        layer_id: None,
        priority: None,
        example_recipe: None,
        example_version: None,
    };

    let old = authority(CompatibilityFixtureRole::OldestPolicyCandidate, 37);
    let old_build = old.snapshot.environment.build_directory.value().unwrap();
    let old_tool = old
        .snapshot
        .environment
        .available_tools
        .value()
        .unwrap()
        .iter()
        .find(|tool| tool.id == "bitbake-layers")
        .unwrap()
        .executable
        .as_path();
    assert!(matches!(
        BitBakeLayersCommandPlanner::new(&old, 37, old_build, old_tool)
            .unwrap()
            .operation(&operation),
        Err(BitBakeLayersCompatibilityError::Unavailable {
            capability: CapabilityId::BitBakeLayersCreateAndAddLayer,
            ..
        })
    ));

    let modern = authority(CompatibilityFixtureRole::LatestSupportCandidate, 38);
    let modern_build = modern.snapshot.environment.build_directory.value().unwrap();
    let modern_tool = modern
        .snapshot
        .environment
        .available_tools
        .value()
        .unwrap()
        .iter()
        .find(|tool| tool.id == "bitbake-layers")
        .unwrap()
        .executable
        .as_path();
    assert_eq!(
        BitBakeLayersCommandPlanner::new(&modern, 38, modern_build, modern_tool)
            .unwrap()
            .operation(&operation)
            .unwrap()
            .arguments(),
        ["create-layer", "--add-layer", "/layers/meta-demo"]
    );
}
