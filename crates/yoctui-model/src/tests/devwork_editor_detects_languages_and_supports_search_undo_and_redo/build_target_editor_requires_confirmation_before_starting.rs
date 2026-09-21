use super::*;

#[test]
fn build_target_editor_requires_confirmation_before_starting() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::BeginBuildTargetEdit);
    if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut() {
        editor.text = "target = \"core-image-minimal\"\n".into();
        editor.cursor = editor.text.len();
    }
    let effect = update(&mut app, Action::ConfirmBuildTarget);

    assert_eq!(effect, None);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }))
    );
}
