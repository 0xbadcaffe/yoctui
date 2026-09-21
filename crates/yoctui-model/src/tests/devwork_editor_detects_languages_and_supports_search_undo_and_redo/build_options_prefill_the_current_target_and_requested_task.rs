use super::*;

#[test]
fn build_options_prefill_the_current_target_and_requested_task() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());

    let _ = update(&mut app, Action::OpenBuildOptions);
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::BeginBuildTargetTask(Some("clean".into())));

    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildTarget { editor, task })
            if !editor.editing
                && editor.text.contains("target = \"core-image-minimal\"")
                && editor.selected_text() == Some("core-image-minimal")
                && task.as_deref() == Some("clean")
    ));
}
