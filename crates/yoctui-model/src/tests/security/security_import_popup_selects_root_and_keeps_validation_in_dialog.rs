use super::*;

#[test]
fn security_import_popup_selects_root_and_keeps_validation_in_dialog() {
    let mut state = SecurityState::default();
    let transition = update_security(&mut state, SecurityAction::BeginImport);
    assert!(matches!(
        transition.dialog,
        SecurityDialogUpdate::Open(SecurityDialog::Import { editor, .. })
            if editor.selected_text() == Some("")
    ));

    let transition = update_security(
        &mut state,
        SecurityAction::ConfirmImport("root = \"relative/report.json\"\n".into()),
    );
    assert!(matches!(
        transition.dialog,
        SecurityDialogUpdate::Open(SecurityDialog::Import {
            validation_error: Some(message),
            ..
        }) if message.contains("absolute")
    ));
    assert!(transition.effect.is_none());
}
