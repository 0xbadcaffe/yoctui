use super::*;

#[test]
fn compatibility_command_getvar_generates_old_and_new_argv_without_unsupported_option() {
    let old = authority(
        1,
        &[(
            CapabilityId::BitBakeGetVar,
            BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION,
        )],
    );
    let old = planner(&old)
        .get_variable("MACHINE", Some("busybox"))
        .unwrap();
    assert_eq!(old.arguments, ["-e", "busybox"]);
    assert!(!old.arguments.iter().any(|argument| argument == "--getvar"));

    let new = authority(
        2,
        &[(
            CapabilityId::BitBakeGetVar,
            BITBAKE_GETVAR_UTILITY_IMPLEMENTATION,
        )],
    );
    let new = planner(&new)
        .get_variable("MACHINE", Some("busybox"))
        .unwrap();
    assert_eq!(new.arguments, ["--value", "--recipe", "busybox", "MACHINE"]);
    assert!(!new.arguments.iter().any(|argument| argument == "-e"));
    assert!(!new.arguments.iter().any(|argument| argument == "--getvar"));

    let mut missing_tool = authority(
        3,
        &[(
            CapabilityId::BitBakeGetVar,
            BITBAKE_GETVAR_UTILITY_IMPLEMENTATION,
        )],
    );
    missing_tool.snapshot.environment.available_tools = AuthoritativeValue::Unknown;
    let missing_tool = missing_tool.normalize().unwrap();
    assert!(matches!(
        planner(&missing_tool).get_variable("MACHINE", None),
        Err(BitBakeCommandAuthorizationError::ToolIdentityUnknown {
            tool: CapabilityToolId::BitBakeGetVar
        })
    ));
    assert!(matches!(
        BitBakeCommandPlanner::new(&missing_tool, 2, Path::new("/work/build")),
        Err(BitBakeCommandAuthorizationError::StaleGeneration { .. })
    ));
}
