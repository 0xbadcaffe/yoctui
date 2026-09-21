use super::*;

#[test]
fn compatibility_ui_model_action_availability_preserves_exact_workspace_projection() {
    let compatibility = state_with(authority(1));
    let available = compatibility_ui_action_availability(
        &compatibility,
        &WorkspaceEffectRequirement::Capabilities {
            all: vec![CapabilityId::BitBakeGetVar],
            any: Vec::new(),
        },
    );
    assert!(available.enabled);
    assert_eq!(
        available.state,
        WorkspaceAvailabilityState::AvailableWithLimitations
    );
    assert!(available.reasons[0].contains("fallback"));
    assert_eq!(available.implementations[0].0, CapabilityId::BitBakeGetVar);

    let denied = compatibility_ui_action_availability(
        &compatibility,
        &WorkspaceEffectRequirement::Capabilities {
            all: vec![CapabilityId::DevtoolUpgrade],
            any: Vec::new(),
        },
    );
    assert!(!denied.enabled);
    assert_eq!(denied.state, WorkspaceAvailabilityState::Unavailable);
    assert_eq!(
        denied.reasons,
        ["Current Devtool does not expose the upgrade subcommand."]
    );
}
