use super::*;

#[test]
fn build_environment_clone_editor_requires_review_before_emitting_clone_effect() {
    let mut app = App::new_unconfigured(8, 512);
    let _ = update(&mut app, Action::OpenBuildEnvironmentCloneEditor);
    if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut() {
        editor.text = "repository = \"https://git.yoctoproject.org/poky\"\ndestination = \"/tmp/poky\"\nrevision = \"\"\nbuild = \"/tmp/poky/build\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::ReviewBuildEnvironmentClone);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildEnvironmentCloneReview(_))
    ));
    assert!(matches!(
        update(&mut app, Action::ConfirmBuildEnvironmentClone),
        Some(Effect::CloneBuildEnvironment(_))
    ));
}
