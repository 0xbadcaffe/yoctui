use super::*;

#[test]
fn compatibility_recipetool_rejects_stale_environment_executable_and_cross_subcommand() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/scripts/recipetool");
    let authority = authority(
        build,
        executable,
        9,
        &[(
            CapabilityId::RecipetoolCreate,
            RECIPETOOL_CREATE_IMPLEMENTATION,
        )],
        &[],
    );
    assert!(matches!(
        RecipetoolCommandPlanner::new(&authority, 8, build, executable),
        Err(RecipetoolCompatibilityError::StaleGeneration { .. })
    ));
    assert!(matches!(
        RecipetoolCommandPlanner::new(&authority, 9, Path::new("/other"), executable),
        Err(RecipetoolCompatibilityError::EnvironmentMismatch)
    ));
    assert!(matches!(
        RecipetoolCommandPlanner::new(&authority, 9, build, Path::new("/usr/bin/recipetool")),
        Err(RecipetoolCompatibilityError::ExecutableMismatch)
    ));
    let planner = RecipetoolCommandPlanner::new(&authority, 9, build, executable).unwrap();
    assert!(matches!(
        planner.operation(&appendfile()),
        Err(RecipetoolCompatibilityError::CapabilityMissing {
            capability: CapabilityId::RecipetoolAppendFile,
        })
    ));
}
