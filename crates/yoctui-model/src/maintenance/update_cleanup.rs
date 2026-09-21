fn update_cleanup(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::OpenCleanupForm
            if state.view == MaintenanceView::Sstate
                && state.capability.snapshot().is_some_and(|snapshot| {
                    snapshot.supports(MaintenanceTool::SstateCacheManagement)
                }) =>
        {
            if let Some(snapshot) = state.capability.snapshot()
                && let Ok(draft) = MaintenanceCleanupDraft::from_metadata(&snapshot.metadata)
            {
                let stamps = if draft.stamps_dirs.is_empty() {
                    "none".to_owned()
                } else {
                    draft
                        .stamps_dirs
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let mut editor = PopupEditor::new(format!(
                    "# protected sstate candidate discovery\n# Cache (read-only): {}\n# Stamps (read-only): {stamps}\n# Context comments are informational; current capability metadata is authoritative.\nduplicates = true\norphans = false\nunreferenced_by_stamps = false\njobs = 1\n",
                    draft.cache_dir.display()
                ));
                let _ = editor.select_toml_value("duplicates");
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::CleanupToml {
                            editor,
                            validation_error: None,
                        },
                    )),
                    ..MaintenanceTransition::none()
                };
            }
        }
        MaintenanceAction::ConfirmCleanupToml(document) => {
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
                    .ok_or_else(|| "Sstate capability is unavailable.".to_owned())?;
                let mut draft = MaintenanceCleanupDraft::from_metadata(&snapshot.metadata)
                    .map_err(str::to_owned)?;
                draft.duplicates = boolean("duplicates")?;
                draft.orphans = boolean("orphans")?;
                draft.unreferenced_by_stamps = boolean("unreferenced_by_stamps")?;
                draft.jobs = get("jobs")?;
                draft.request().map_err(str::to_owned)
            })();
            match parsed {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewCleanup {
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
                            MaintenanceDialog::CleanupToml {
                                editor: PopupEditor::new(document),
                                validation_error: Some(message),
                            },
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        MaintenanceAction::UpdateCleanupForm(draft) if draft.is_bounded() => {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::CleanupForm(
                    draft,
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmCleanupForm(mut draft) if draft.is_bounded() => {
            match draft.request() {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewCleanup {
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
                            MaintenanceDialog::CleanupForm(draft),
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
