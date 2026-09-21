use super::*;

#[test]
fn ux_keymap_migrates_legacy_alias_fields_and_rejects_future_schema() {
    let legacy: KeymapPreferences = toml::from_str(
        "schema_version = 0\n[[bindings]]\naction = 'navigate.logs'\nkeys = ['z', 'g l']\n",
    )
    .unwrap();
    let migrated = legacy.migrate().unwrap();
    assert_eq!(migrated.schema_version, KEYMAP_SCHEMA_VERSION);
    let effective = EffectiveKeymap::from_preferences(&migrated).unwrap();
    assert_eq!(
        effective
            .bindings_for_action(OperatorActionId::new("navigate.logs"))
            .map(|binding| binding.sequence.to_string())
            .collect::<Vec<_>>(),
        ["z", "g l"]
    );

    assert!(matches!(
        KeymapPreferences {
            schema_version: KEYMAP_SCHEMA_VERSION + 1,
            overrides: Vec::new(),
        }
        .migrate(),
        Err(KeymapError::UnsupportedSchema(_))
    ));
}
