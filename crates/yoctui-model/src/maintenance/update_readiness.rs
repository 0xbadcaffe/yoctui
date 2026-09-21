fn update_readiness(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::OpenReadinessForm
            if state.view == MaintenanceView::Sstate
                && state
                    .capability
                    .snapshot()
                    .is_some_and(|snapshot| snapshot.supports(MaintenanceTool::OeCheckSstate)) =>
        {
            let mut editor = PopupEditor::new(
                "# exact sstate readiness request\ntargets = \"\"\nmode = \"isolated_tmpdir\"\noutput = \"\"\nlog = \"\"\ntimeout = 3600\n"
                    .into(),
            );
            let _ = editor.select_toml_value("targets");
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::ReadinessToml {
                    editor,
                    validation_error: None,
                })),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmReadinessToml(document) => {
            let parsed = (|| {
                let fields = popup_toml_fields(&document)?;
                let get = |key: &str| {
                    fields
                        .get(key)
                        .cloned()
                        .ok_or_else(|| format!("Missing `{key}`."))
                };
                let mode = match get("mode")?.as_str() {
                    "isolated_tmpdir" => SstateReadinessMode::IsolatedTmpdir,
                    "same_tmpdir" => SstateReadinessMode::SameTmpdir,
                    _ => {
                        return Err("`mode` must be `isolated_tmpdir` or `same_tmpdir`.".to_owned());
                    }
                };
                let draft = MaintenanceReadinessDraft {
                    field: MaintenanceReadinessField::Targets,
                    targets: get("targets")?,
                    mode,
                    output: get("output")?,
                    log: get("log")?,
                    timeout: get("timeout")?,
                    validation: None,
                };
                draft.request().map_err(str::to_owned)
            })();
            match parsed {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewReadiness {
                            capability_request,
                            request,
                        }),
                        dialog: MaintenanceDialogUpdate::Close,
                        notification: None,
                    };
                }
                Err(message) => {
                    return MaintenanceTransition {
                        dialog: MaintenanceDialogUpdate::Open(Box::new(
                            MaintenanceDialog::ReadinessToml {
                                editor: PopupEditor::new(document),
                                validation_error: Some(message),
                            },
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        MaintenanceAction::UpdateReadinessForm(draft) if draft.is_bounded() => {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::ReadinessForm(
                    draft,
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmReadinessForm(mut draft) if draft.is_bounded() => {
            match draft.request() {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewReadiness {
                            capability_request,
                            request,
                        }),
                        dialog: MaintenanceDialogUpdate::Close,
                        notification: None,
                    };
                }
                Err(message) => {
                    draft.validation = Some(message.into());
                    return MaintenanceTransition {
                        dialog: MaintenanceDialogUpdate::Open(Box::new(
                            MaintenanceDialog::ReadinessForm(draft),
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        _ => {}
    }
    MaintenanceTransition::none()
}
