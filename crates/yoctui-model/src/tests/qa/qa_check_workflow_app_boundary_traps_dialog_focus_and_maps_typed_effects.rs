use super::*;

#[test]
fn qa_check_workflow_app_boundary_traps_dialog_focus_and_maps_typed_effects() {
    let mut app = App::new(10, 1_000);
    app.screen = crate::Screen::Qa;
    app.focus = FocusTarget::Workspace;
    assert_eq!(
        update(&mut app, Action::Qa(QaAction::InspectCapability)),
        Some(Effect::Qa(QaEffect::InspectCapability { scope: None }))
    );
    let _ = update(
        &mut app,
        Action::Qa(QaAction::CapabilityLoaded(capability())),
    );
    let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedCheck));
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::Qa(QaDialog::Operation(_)))
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::Qa(QaAction::CancelDialog));
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, FocusTarget::Workspace);

    let _ = update(&mut app, Action::Qa(QaAction::BeginImport));
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::Qa(QaDialog::Import { editor, .. }))
            if editor.selected_text() == Some("")
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);

    let transition = update_qa(
        &mut app.qa,
        QaAction::ConfirmImport("root = \"relative/report.json\"\n".into()),
    );
    assert!(matches!(
        transition.dialog,
        QaDialogUpdate::Open(dialog)
            if matches!(*dialog, QaDialog::Import {
                validation_error: Some(ref message),
                ..
            } if message.contains("absolute"))
    ));
    assert!(transition.effect.is_none());
}
