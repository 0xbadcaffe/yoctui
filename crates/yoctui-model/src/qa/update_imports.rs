fn update_imports(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        QaAction::BeginImport => QaTransition {
            dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                editor: {
                    let mut editor = PopupEditor::new(popup_toml_document(
                        "root",
                        "",
                        Some("normalized absolute QA report or bounded directory"),
                    ));
                    let _ = editor.select_toml_value("root");
                    editor
                },
                validation_error: None,
            })),
            ..QaTransition::none()
        },
        QaAction::UpdateImport(document) if document.len() <= MAX_QA_TEXT_BYTES => QaTransition {
            dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                editor: PopupEditor::new(document),
                validation_error: None,
            })),
            ..QaTransition::none()
        },
        QaAction::UpdateImport(_) => QaTransition::none(),
        QaAction::ConfirmImport(document) => {
            let root = match popup_toml_value(&document, "root") {
                Ok(root) => PathBuf::from(root),
                Err(message) => {
                    return QaTransition {
                        dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                            editor: PopupEditor::new(document),
                            validation_error: Some(message),
                        })),
                        ..QaTransition::none()
                    };
                }
            };
            if !absolute_normal_path(&root) {
                return QaTransition {
                    dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                        editor: PopupEditor::new(document),
                        validation_error: Some("`root` must be a normalized absolute path.".into()),
                    })),
                    ..QaTransition::none()
                };
            }
            match begin_report_request(state, vec![root]) {
                Ok(effect) => QaTransition {
                    effect: Some(effect),
                    dialog: QaDialogUpdate::Close,
                    notification: None,
                },
                Err(message) => QaTransition {
                    dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                        editor: PopupEditor::new(document),
                        validation_error: Some(message.into()),
                    })),
                    ..QaTransition::none()
                },
            }
        }
        QaAction::CancelDialog => {
            state.pending_operation = None;
            state.pending_layer_operation = None;
            QaTransition {
                dialog: QaDialogUpdate::Close,
                ..QaTransition::none()
            }
        }
        _ => QaTransition::none(),
    }
}
