use super::*;

#[test]
fn ux_keymap_defaults_are_valid_scoped_and_exportable() {
    let keymap = EffectiveKeymap::default();
    assert!(keymap.bindings.len() >= 20);
    assert!(keymap.report().starts_with("yoctui keymap schema 1\n"));
    let select_image = keymap
        .bindings
        .iter()
        .find(|binding| binding.action_id.as_str() == "navigate.select-image")
        .unwrap();
    assert_eq!(
        select_image.scope,
        KeymapScope::Workspace(WorkspaceDestination::Images)
    );
    assert_eq!(select_image.sequence.to_string(), "i");
    assert!(keymap.bindings.iter().any(|binding| {
        binding.action_id.as_str() == "configure.bbmask" && binding.sequence.to_string() == "x e"
    }));
}

#[test]
fn modifier_keymap_round_trips_and_exposes_portable_defaults() {
    for letter in 'a'..='z' {
        let stroke = KeyStroke::Alt(letter);
        assert_eq!(stroke.to_string().parse::<KeyStroke>().unwrap(), stroke);
        let sequence = KeySequence::single(stroke);
        let encoded = serde_json::to_string(&sequence).unwrap();
        assert_eq!(
            serde_json::from_str::<KeySequence>(&encoded).unwrap(),
            sequence
        );
    }
    for invalid in ["Alt+", "Alt+enter", "Alt+1", "Alt+gg", "Ctrl+Alt+f"] {
        assert!(invalid.parse::<KeyStroke>().is_err());
    }
    let map = EffectiveKeymap::default();
    for (id, shortcut) in [("build.image", "Alt+b"), ("tools.gitui", "Alt+g")] {
        assert!(
            map.bindings
                .iter()
                .any(|binding| binding.action_id.as_str() == id
                    && binding.sequence.to_string() == shortcut),
            "{id}: {}",
            map.report()
        );
    }
    for definition in crate::operator_action_catalog() {
        assert!(
            !definition
                .shortcut
                .split('/')
                .any(|token| token.len() == 1 && token.as_bytes()[0].is_ascii_uppercase()),
            "{}: {}",
            definition.id.as_str(),
            definition.shortcut
        );
    }
}
