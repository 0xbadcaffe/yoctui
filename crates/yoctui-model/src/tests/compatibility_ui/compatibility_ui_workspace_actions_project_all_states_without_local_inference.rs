use super::*;

#[test]
fn compatibility_ui_workspace_actions_project_all_states_without_local_inference() {
    let compatibility = state_with(authority(9));
    let configuration = compatibility_ui_workspace_action_presentations(
        &compatibility,
        crate::WorkspaceDestination::Configuration,
    );
    let getvar = configuration
        .iter()
        .find(|action| action.id == "configuration.getvar")
        .unwrap();
    assert!(getvar.availability.enabled);
    assert_eq!(
        getvar.availability.state,
        WorkspaceAvailabilityState::AvailableWithLimitations
    );
    assert!(getvar.availability.reasons[0].contains("fallback"));
    let local = configuration
        .iter()
        .find(|action| action.id == "configuration.inspect")
        .unwrap();
    assert!(local.availability.enabled);
    assert_eq!(
        local.availability.state,
        WorkspaceAvailabilityState::Available
    );

    let devtool = compatibility_ui_workspace_action_presentations(
        &compatibility,
        crate::WorkspaceDestination::Devtool,
    );
    let upgrade = devtool
        .iter()
        .find(|action| action.id == "devtool.upgrade")
        .unwrap();
    assert!(!upgrade.availability.enabled);
    assert_eq!(
        upgrade.availability.state,
        WorkspaceAvailabilityState::Unavailable
    );
    assert_eq!(
        upgrade.availability.exact_reason().as_deref(),
        Some("Current Devtool does not expose the upgrade subcommand.")
    );
    let absent = compatibility_ui_workspace_action_presentations(
        &WorkspaceCompatibilityState::default(),
        crate::WorkspaceDestination::Images,
    );
    assert_eq!(
        absent
            .iter()
            .find(|action| action.id == "images.qemu")
            .unwrap()
            .availability
            .state,
        WorkspaceAvailabilityState::Unknown
    );
    assert!(
        absent
            .iter()
            .find(|action| action.id == "images.device_write")
            .unwrap()
            .availability
            .enabled
    );
}
