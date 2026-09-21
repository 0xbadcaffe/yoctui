use super::*;

#[test]
fn compatibility_command_getvar_shared_fixtures_select_fallback_and_utility_argv() {
    let old = fixture_authority(CompatibilityFixtureRole::OldestPolicyCandidate, 31);
    let old_command = fixture_planner(&old)
        .get_variable("MACHINE", Some("busybox"))
        .unwrap();
    assert_eq!(old_command.arguments, ["-e", "busybox"]);
    assert_eq!(
        old_command.executable,
        Path::new("/fixtures/oldest-policy-candidate/bin/bitbake")
    );
    assert_eq!(
        old_command.implementation,
        BITBAKE_GETVAR_ENVIRONMENT_IMPLEMENTATION
    );

    let modern = fixture_authority(CompatibilityFixtureRole::LatestSupportCandidate, 32);
    let modern_command = fixture_planner(&modern)
        .get_variable("MACHINE", Some("busybox"))
        .unwrap();
    assert_eq!(
        modern_command.arguments,
        ["--value", "--recipe", "busybox", "MACHINE"]
    );
    assert_eq!(
        modern_command.implementation,
        BITBAKE_GETVAR_UTILITY_IMPLEMENTATION
    );
    assert_eq!(
        modern_command.executable,
        Path::new("/fixtures/latest-support-candidate/bin/bitbake-getvar")
    );
    assert!(
        !modern_command
            .arguments
            .iter()
            .any(|argument| argument == "--getvar")
    );
}
