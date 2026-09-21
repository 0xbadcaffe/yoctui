use super::*;

#[test]
fn security_workflow_maps_workspace_search_and_modal_keys_without_leakage() {
    assert_eq!(
        security_workspace_action(SecurityView::Cves, false, false, Input::Char('V')),
        Some(Action::Security(SecurityAction::BeginCveCheck))
    );
    assert_eq!(
        security_workspace_action(SecurityView::Sbom, false, false, Input::Down),
        Some(Action::Security(SecurityAction::SelectReport(1)))
    );
    assert_eq!(
        security_workspace_action(SecurityView::Sbom, true, false, Input::Down),
        Some(Action::Security(SecurityAction::SelectComponent(1)))
    );
    assert_eq!(
        security_workspace_action(SecurityView::Cves, false, true, Input::Char('x')),
        Some(Action::Security(SecurityAction::AppendQuery('x')))
    );
    assert_eq!(
        security_workspace_action(SecurityView::Cves, false, true, Input::Char('V')),
        Some(Action::Security(SecurityAction::AppendQuery('V'))),
        "search editing consumes workflow shortcuts"
    );

    let preview = yoctui_model::SecurityOperationPreview {
        id: yoctui_model::SecuritySessionId(7),
        scope: yoctui_model::SecurityScope::Image {
            target: "core-image-minimal".into(),
            machine: "qemux86-64".into(),
            distro: "poky".into(),
        },
        operation: yoctui_model::SecurityOperation::SbomBuild(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("create_recipe_sbom".into()),
            force: false,
        }),
        indexed_arguments: vec!["0: bitbake".into()],
        report_roots: vec!["/build/tmp/deploy/spdx".into()],
    };
    assert_eq!(
        security_dialog_action(&SecurityDialog::Operation(preview.clone()), Input::Enter),
        Some(Action::Security(SecurityAction::ConfirmOperation(preview)))
    );
    let mut import_editor = yoctui_model::PopupEditor::new("root = \"/reports\"\n".into());
    import_editor.select_toml_value("root").unwrap();
    import_editor.editing = true;
    assert_eq!(
        security_dialog_action(
            &SecurityDialog::Import {
                editor: import_editor,
                validation_error: None,
            },
            Input::Char('V')
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('V'))),
        "modal text editing does not leak CVE launch"
    );
    assert_eq!(
        security_dialog_action(
            &SecurityDialog::Cancellation(yoctui_model::SecuritySessionId(7)),
            Input::Esc
        ),
        Some(Action::Security(SecurityAction::CancelDialog))
    );
}
