fn update_build_history(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::OpenBuildHistoryForm
            if state.view == MaintenanceView::Release
                && state.capability.snapshot().is_some_and(|snapshot| {
                    snapshot.supports(MaintenanceTool::BuildHistoryDiff)
                        && snapshot.metadata.buildhistory_dir.is_some()
                }) =>
        {
            if let Some(snapshot) = state.capability.snapshot()
                && let Ok(draft) = MaintenanceBuildHistoryDraft::from_metadata(&snapshot.metadata)
            {
                let mut editor = PopupEditor::new(format!(
                    "# build-history comparison\n# Repository (read-only): {}\nfrom_revision = \"\"\nto_revision = \"\"\nreport_version = false\nreport_all = false\nsignatures = false\nsignature_diff = false\nexclude_paths = \"\"\nno_colour = false\n",
                    draft.repository.display(),
                ));
                let _ = editor.select_toml_value("from_revision");
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::BuildHistoryToml {
                            editor,
                            validation_error: None,
                        },
                    )),
                    ..MaintenanceTransition::none()
                };
            }
        }
        MaintenanceAction::ConfirmBuildHistoryToml(document) => {
            let parsed = (|| {
                let fields = popup_toml_fields(&document)?;
                let get = |key: &str| {
                    fields
                        .get(key)
                        .cloned()
                        .ok_or_else(|| format!("Missing `{key}`."))
                };
                let boolean = |key: &str| match get(key)?.as_str() {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    _ => Err(format!("`{key}` must be `true` or `false`.")),
                };
                let snapshot = state
                    .capability
                    .snapshot()
                    .ok_or_else(|| "Build-history capability is unavailable.".to_owned())?;
                let mut draft = MaintenanceBuildHistoryDraft::from_metadata(&snapshot.metadata)
                    .map_err(str::to_owned)?;
                draft.from_revision = get("from_revision")?;
                draft.to_revision = get("to_revision")?;
                draft.report_version = boolean("report_version")?;
                draft.report_all = boolean("report_all")?;
                draft.signatures = boolean("signatures")?;
                draft.signature_diff = boolean("signature_diff")?;
                draft.exclude_paths = get("exclude_paths")?;
                draft.no_colour = boolean("no_colour")?;
                draft.request().map_err(str::to_owned)
            })();
            match parsed {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewBuildHistoryComparison {
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
                            MaintenanceDialog::BuildHistoryToml {
                                editor: PopupEditor::new(document),
                                validation_error: Some(message),
                            },
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        MaintenanceAction::UpdateBuildHistoryForm(draft) if draft.is_bounded() => {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(
                    MaintenanceDialog::BuildHistoryForm(draft),
                )),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmBuildHistoryForm(mut draft) if draft.is_bounded() => {
            match draft.request() {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewBuildHistoryComparison {
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
                            MaintenanceDialog::BuildHistoryForm(draft),
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
