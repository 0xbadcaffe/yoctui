use super::*;

#[test]
fn ux_keymap_rejects_collision_prefix_reserved_unknown_scope_and_unreachable() {
    let collision = KeymapPreferences {
        schema_version: KEYMAP_SCHEMA_VERSION,
        overrides: vec![override_binding(
            "navigate.logs",
            KeymapScope::Global,
            &["e"],
        )],
    };
    assert!(matches!(
        EffectiveKeymap::from_preferences(&collision),
        Err(KeymapError::Collision { .. })
    ));

    let prefix = KeymapPreferences {
        schema_version: KEYMAP_SCHEMA_VERSION,
        overrides: vec![override_binding(
            "navigate.logs",
            KeymapScope::Global,
            &["Esc x"],
        )],
    };
    assert!(matches!(
        EffectiveKeymap::from_preferences(&prefix),
        Err(KeymapError::Collision { .. })
    ));

    let reserved = KeymapPreferences {
        schema_version: KEYMAP_SCHEMA_VERSION,
        overrides: vec![override_binding(
            "navigate.logs",
            KeymapScope::Global,
            &["Ctrl+B l"],
        )],
    };
    assert!(matches!(
        EffectiveKeymap::from_preferences(&reserved),
        Err(KeymapError::ReservedTerminalPrefix { .. })
    ));

    let unknown = KeymapPreferences {
        schema_version: KEYMAP_SCHEMA_VERSION,
        overrides: vec![override_binding(
            "missing.action",
            KeymapScope::Global,
            &["z"],
        )],
    };
    assert!(matches!(
        EffectiveKeymap::from_preferences(&unknown),
        Err(KeymapError::UnknownAction(_))
    ));

    let wrong_scope = KeymapPreferences {
        schema_version: KEYMAP_SCHEMA_VERSION,
        overrides: vec![override_binding(
            "navigate.logs",
            KeymapScope::Workspace(WorkspaceDestination::Logs),
            &["z"],
        )],
    };
    assert!(matches!(
        EffectiveKeymap::from_preferences(&wrong_scope),
        Err(KeymapError::ScopeMismatch { .. })
    ));

    let unreachable = KeymapPreferences {
        schema_version: KEYMAP_SCHEMA_VERSION,
        overrides: vec![override_binding("help.open", KeymapScope::Global, &[])],
    };
    assert!(matches!(
        EffectiveKeymap::from_preferences(&unreachable),
        Err(KeymapError::UnreachableCriticalAction(action)) if action == "help.open"
    ));
}
