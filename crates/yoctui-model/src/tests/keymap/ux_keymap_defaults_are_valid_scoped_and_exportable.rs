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
