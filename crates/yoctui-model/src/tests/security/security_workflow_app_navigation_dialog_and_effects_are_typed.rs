use super::*;

#[test]
fn security_workflow_app_navigation_dialog_and_effects_are_typed() {
    let mut app = App::new(20, 4_000);
    assert_eq!(
        update(&mut app, Action::Open(Screen::Security)),
        Some(Effect::Security(SecurityEffect::InspectCapability))
    );
    assert_eq!(app.screen, Screen::Security);
    assert_eq!(app.focus, FocusTarget::Navigator);
    let _ = update(
        &mut app,
        Action::Security(SecurityAction::CapabilityLoaded(capability())),
    );
    let _ = update(
        &mut app,
        Action::Security(SecurityAction::BeginSbomGeneration),
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::Security(SecurityDialog::Operation(
            SecurityOperationPreview {
                operation: SecurityOperation::SbomBuild(BuildRequest {
                    task: Some(task),
                    ..
                }),
                ..
            }
        ))) if task == "create_recipe_sbom"
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let preview = match app.active_dialog().cloned().unwrap() {
        Dialog::Security(SecurityDialog::Operation(preview)) => preview,
        _ => unreachable!(),
    };
    assert!(matches!(
        update(
            &mut app,
            Action::Security(SecurityAction::ConfirmOperation(preview))
        ),
        Some(Effect::Security(SecurityEffect::StartBuild { .. }))
    ));
    assert_eq!(app.focus, FocusTarget::Navigator);
}
