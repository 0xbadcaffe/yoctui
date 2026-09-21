use super::*;

#[test]
fn compatibility_ui_action_catalog_preserves_inspection_gating_and_exact_fallback() {
    let absent = WorkspaceCompatibilityState::default();
    let layers = compatibility_ui_destination_action_availability(&absent, Screen::Layers);
    assert!(layers.enabled);
    assert_eq!(layers.state, WorkspaceAvailabilityState::Unknown);
    assert!(
        layers
            .exact_reason()
            .unwrap()
            .contains("current environment capability snapshot")
    );

    let build = compatibility_ui_command_action_availability(&absent, CommandId::BuildImage);
    assert!(!build.enabled);
    assert_eq!(build.state, WorkspaceAvailabilityState::Unknown);

    let local = compatibility_ui_command_action_availability(&absent, CommandId::ChooseTheme);
    assert!(local.enabled);
    assert_eq!(local.state, WorkspaceAvailabilityState::Available);
    assert!(local.exact_reason().is_none());

    let current = state_with(authority(7));
    let configuration =
        compatibility_ui_destination_action_availability(&current, Screen::Configuration);
    assert!(configuration.enabled);
    assert_eq!(
        configuration.state,
        WorkspaceAvailabilityState::AvailableWithLimitations
    );
    assert_eq!(
        configuration.implementations,
        [(
            CapabilityId::BitBakeGetVar,
            "bitbake.getvar.environment-fallback".into()
        )]
    );
    assert_eq!(
        configuration.exact_reason().as_deref(),
        Some("Native getvar is absent; the environment dump fallback is selected.")
    );
}
