use super::*;

#[test]
fn build_environment_toml_editor_applies_profile_from_popup() {
    let mut app = App::new_unconfigured(8, 512);
    let _ = update(&mut app, Action::OpenBuildEnvironmentEditor);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildEnvironmentEditor(editor)) if !editor.editing
    ));
    if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut() {
        editor.editing = true;
        editor.text = "source = \"/src/poky\"\nbuild = \"/src/build\"\nscript = \"/src/poky/oe-init-build-env\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::ApplyBuildEnvironmentEditor);
    assert!(matches!(
        app.build_environment,
        BuildEnvironmentState::Configured(_)
    ));
}
