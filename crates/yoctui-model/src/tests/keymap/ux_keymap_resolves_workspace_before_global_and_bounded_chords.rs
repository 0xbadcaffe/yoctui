use super::*;

#[test]
fn ux_keymap_resolves_workspace_before_global_and_bounded_chords() {
    let keymap = EffectiveKeymap::default();
    let mut state = KeymapChordState::default();
    assert_eq!(
        keymap.resolve_input(
            &mut state,
            WorkspaceDestination::Images,
            KeyStroke::Char('i')
        ),
        KeymapResolution::Activated(OperatorActionId::new("navigate.select-image"))
    );
    assert_eq!(
        keymap.resolve_input(
            &mut state,
            WorkspaceDestination::Tasks,
            KeyStroke::Char('i')
        ),
        KeymapResolution::Activated(OperatorActionId::new("navigate.images"))
    );
    assert_eq!(
        keymap.resolve_input(
            &mut state,
            WorkspaceDestination::Configuration,
            KeyStroke::Char('x')
        ),
        KeymapResolution::Pending
    );
    assert!(state.is_pending());
    assert_eq!(
        keymap.resolve_input(
            &mut state,
            WorkspaceDestination::Configuration,
            KeyStroke::Char('e')
        ),
        KeymapResolution::Activated(OperatorActionId::new("configure.bbmask"))
    );
    assert!(!state.is_pending());
}
