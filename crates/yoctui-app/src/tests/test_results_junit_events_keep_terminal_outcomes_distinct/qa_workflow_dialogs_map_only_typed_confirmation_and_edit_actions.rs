use super::*;

#[test]
fn qa_workflow_dialogs_map_only_typed_confirmation_and_edit_actions() {
    let scope = yoctui_model::QaScope::new(yoctui_model::RecipeIdentity {
        name: "linux-yocto".into(),
        file: "/layers/meta/recipes-kernel/linux/linux-yocto.bb".into(),
    })
    .unwrap();
    let preview = yoctui_model::QaOperationPreview {
        id: yoctui_model::QaOperationId(7),
        check: yoctui_model::QaCheckId::new("kernel-config".into()).unwrap(),
        family: yoctui_model::QaCheckFamily::KernelConfiguration,
        scope,
        request: BuildRequest {
            targets: vec!["linux-yocto".into()],
            task: Some("kernel_configcheck".into()),
            force: false,
        },
        indexed_arguments: vec!["0: bitbake".into()],
        report_roots: vec!["/build/tmp/log/qa".into()],
        limitations: vec![],
    };
    assert_eq!(
        qa_dialog_action(&QaDialog::Operation(preview.clone()), Input::Enter),
        Some(Action::Qa(QaAction::ConfirmOperation(preview)))
    );
    assert_eq!(
        {
            let mut editor = yoctui_model::PopupEditor::new("root = \"/reports\"\n".into());
            editor.select_toml_value("root").unwrap();
            editor.editing = true;
            qa_dialog_action(
                &QaDialog::Import {
                    editor,
                    validation_error: None,
                },
                Input::Char('r'),
            )
        },
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('r'))),
        "modal text editing does not leak QA run"
    );
    assert_eq!(
        qa_dialog_action(
            &QaDialog::Cancellation {
                session: yoctui_model::QaSessionId(3),
                background_job: BackgroundJobId(9),
            },
            Input::Enter,
        ),
        Some(Action::Qa(QaAction::ConfirmCancellation(
            yoctui_model::QaSessionId(3)
        )))
    );
    let layer_preview = yoctui_model::QaLayerOperationPreview {
        id: yoctui_model::QaLayerOperationId(4),
        check: yoctui_model::QaCheckId::new("yocto-check-layer".into()).unwrap(),
        layer: yoctui_model::QaLayerIdentity::new("meta".into(), "/layers/meta".into()).unwrap(),
        executable: yoctui_model::QaExecutableIdentity::new(
            "/poky/scripts/yocto-check-layer".into(),
            10,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap(),
        arguments: vec!["--layer".into(), "/layers/meta".into()],
        indexed_arguments: vec!["0: /poky/scripts/yocto-check-layer".into()],
        report_roots: vec![],
        limitations: vec![],
    };
    assert_eq!(
        qa_dialog_action(
            &QaDialog::LayerOperation(layer_preview.clone()),
            Input::Enter
        ),
        Some(Action::Qa(QaAction::ConfirmLayerOperation(layer_preview)))
    );
    assert_eq!(
        qa_dialog_action(
            &QaDialog::LayerCancellation(yoctui_model::QaLayerSessionId(4)),
            Input::Enter,
        ),
        Some(Action::Qa(QaAction::ConfirmLayerCancellation(
            yoctui_model::QaLayerSessionId(4)
        )))
    );
    assert_eq!(
        qa_dialog_action(
            &QaDialog::Import {
                editor: yoctui_model::PopupEditor::new("root = \"\"\n".into()),
                validation_error: None,
            },
            Input::Tab
        ),
        None
    );
}
