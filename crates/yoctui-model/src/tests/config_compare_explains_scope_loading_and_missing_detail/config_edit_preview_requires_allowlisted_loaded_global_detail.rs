use super::*;

#[test]
fn config_edit_preview_requires_allowlisted_loaded_global_detail() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Inspector;
    app.workspace.build_dir = Some("/build".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity: identity.clone(),
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::BeginConfigEdit);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ConfigEdit { identity: selected, editor })
            if selected == &identity
                && !editor.editing
                && editor.text.contains("value = \"qemux86-64\"")
                && editor.selected_text() == Some("qemux86-64")
    ));
    assert!(matches!(
        update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Copy)
        ),
        Some(Effect::CopyToClipboard(value)) if value == "qemux86-64"
    ));
    if let Some(Dialog::ConfigEdit { editor, .. }) = app.active_dialog_mut() {
        editor.text = "# MACHINE\nvalue = \"qemux86-64\\\"\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewConfigEdit);
    let Some(Dialog::ConfigEditConfirmation(request)) = app.active_dialog() else {
        panic!("confirmation was not opened");
    };
    assert_eq!(request.destination, PathBuf::from("/build/conf/local.conf"));
    assert_eq!(request.assignment, "MACHINE = \"qemux86-64\\\"\"");
    let expected = request.clone();
    assert_eq!(
        update(&mut app, Action::ConfirmConfigEdit),
        Some(Effect::WriteConfigAssignment(expected))
    );
    assert_eq!(app.focus, FocusTarget::Navigator);
}
