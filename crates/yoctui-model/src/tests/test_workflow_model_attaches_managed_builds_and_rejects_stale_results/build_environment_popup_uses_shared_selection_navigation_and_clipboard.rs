use super::*;

#[test]
fn build_environment_popup_uses_shared_selection_navigation_and_clipboard() {
    let mut app = App::new_unconfigured(8, 512);
    let _ = update(
        &mut app,
        Action::ConfigureBuildEnvironment(BuildEnvironmentProfile {
            source_dir: "/old/source".into(),
            build_dir: "/old/build".into(),
            init_script: "/old/source/oe-init-build-env".into(),
        }),
    );
    let _ = update(&mut app, Action::OpenBuildEnvironmentEditor);
    assert!(matches!(
        update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Copy)
        ),
        Some(Effect::CopyToClipboard(value)) if value == "/old/source"
    ));
    let _ = update(
        &mut app,
        Action::EditActivePopup(PopupEditorCommand::ToggleInsert),
    );
    for character in "/new/source".chars() {
        let _ = update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Insert(character)),
        );
    }
    let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog() else {
        panic!("build environment editor");
    };
    assert!(editor.text.starts_with("source = \"/new/source\""));
}
