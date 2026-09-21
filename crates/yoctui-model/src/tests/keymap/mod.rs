use super::*;

fn override_binding(action_id: &str, scope: KeymapScope, sequences: &[&str]) -> KeymapOverride {
    KeymapOverride {
        action_id: action_id.into(),
        scope,
        sequences: sequences
            .iter()
            .map(|sequence| sequence.parse().unwrap())
            .collect(),
    }
}

mod ux_keymap_defaults_are_valid_scoped_and_exportable;

mod ux_keymap_resolves_workspace_before_global_and_bounded_chords;

mod ux_keymap_rejects_collision_prefix_reserved_unknown_scope_and_unreachable;

mod ux_keymap_migrates_legacy_alias_fields_and_rejects_future_schema;
