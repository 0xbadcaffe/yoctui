fn update_imports(state: &mut SecurityState, action: SecurityAction) -> SecurityTransition {
    match action {
        SecurityAction::BeginImport => SecurityTransition {
            dialog: SecurityDialogUpdate::Open(SecurityDialog::Import {
                editor: {
                    let mut editor = PopupEditor::new(popup_toml_document(
                        "root",
                        "",
                        Some(
                            "normalized absolute CVE/SPDX report or bounded directory; exact canonical non-symlink path only",
                        ),
                    ));
                    let _ = editor.select_toml_value("root");
                    editor
                },
                validation_error: None,
            }),
            ..SecurityTransition::none()
        },
        SecurityAction::UpdateImport(document) if document.len() <= MAX_SECURITY_TEXT_BYTES => {
            SecurityTransition {
                dialog: SecurityDialogUpdate::Open(SecurityDialog::Import {
                    editor: PopupEditor::new(document),
                    validation_error: None,
                }),
                ..SecurityTransition::none()
            }
        }
        SecurityAction::UpdateImport(_) => SecurityTransition::none(),
        SecurityAction::ConfirmImport(document) => {
            let root = match popup_toml_value(&document, "root") {
                Ok(root) => PathBuf::from(root),
                Err(message) => {
                    return SecurityTransition {
                        dialog: SecurityDialogUpdate::Open(SecurityDialog::Import {
                            editor: PopupEditor::new(document),
                            validation_error: Some(message),
                        }),
                        ..SecurityTransition::none()
                    };
                }
            };
            if !absolute_normal_path(&root) {
                return SecurityTransition {
                    dialog: SecurityDialogUpdate::Open(SecurityDialog::Import {
                        editor: PopupEditor::new(document),
                        validation_error: Some("`root` must be a normalized absolute path.".into()),
                    }),
                    ..SecurityTransition::none()
                };
            }
            match begin_report_request(state, vec![root]) {
                Ok(effect) => SecurityTransition {
                    effect: Some(effect),
                    dialog: SecurityDialogUpdate::Close,
                    notification: None,
                },
                Err(message) => SecurityTransition {
                    dialog: SecurityDialogUpdate::Open(SecurityDialog::Import {
                        editor: PopupEditor::new(document),
                        validation_error: Some(message.into()),
                    }),
                    ..SecurityTransition::none()
                },
            }
        }
        SecurityAction::CancelDialog => SecurityTransition {
            dialog: SecurityDialogUpdate::Close,
            ..SecurityTransition::none()
        },
        _ => SecurityTransition::none(),
    }
}
