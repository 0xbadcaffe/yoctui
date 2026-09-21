use super::*;

#[test]
fn dialog_completion_queues_behind_active_dialog_and_restores_focus_after_both_close() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Navigator;
    let _ = update(&mut app, Action::OpenBuildOptions);
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );

    assert_eq!(
        app.dialogs.iter().collect::<Vec<_>>(),
        vec![&Dialog::BuildOptions, &Dialog::BuildCompletion]
    );
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::DismissBuildCompletion);
    assert_eq!(app.dialogs.len(), 2, "only the active dialog may dismiss");

    let _ = update(&mut app, Action::CloseBuildOptions);
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::DismissBuildCompletion);
    assert!(app.dialogs.is_empty());
    assert_eq!(app.focus, FocusTarget::Navigator);
}
