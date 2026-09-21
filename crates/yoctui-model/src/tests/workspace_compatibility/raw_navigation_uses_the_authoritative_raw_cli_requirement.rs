use super::*;

#[test]
fn raw_navigation_uses_the_authoritative_raw_cli_requirement() {
    assert_eq!(
        workspace_screen_destination(Screen::RawMode),
        WorkspaceDestination::RawMode
    );
    assert_eq!(
        workspace_destination_requirement(WorkspaceDestination::RawMode),
        WorkspaceEffectRequirement::one(CapabilityId::BitBakeRawCli)
    );
    assert_eq!(
        workspace_requirement_availability(
            None,
            &workspace_destination_requirement(WorkspaceDestination::RawMode),
        )
        .state,
        WorkspaceAvailabilityState::Unknown
    );
}
