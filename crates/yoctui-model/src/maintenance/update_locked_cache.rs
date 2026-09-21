fn update_locked_cache(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::OpenLockedCacheForm
            if state.view == MaintenanceView::Release
                && state.capability.snapshot().is_some_and(|snapshot| {
                    snapshot.supports(MaintenanceTool::LockedSignatureCache)
                        && snapshot.metadata.native_lsb.is_some()
                }) =>
        {
            if let Some(snapshot) = state.capability.snapshot()
                && let Ok(draft) = MaintenanceLockedCacheDraft::from_metadata(&snapshot.metadata)
            {
                let mut editor = PopupEditor::new(format!(
                    "# locked-signature cache request\n# Native LSB (read-only): {}\nlocked_signatures = \"\"\ninput_cache = \"\"\noutput_cache = \"\"\nfilter = \"\"\n",
                    draft.native_lsb,
                ));
                let _ = editor.select_toml_value("locked_signatures");
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::LockedCacheToml {
                            editor,
                            validation_error: None,
                        },
                    )),
                    ..MaintenanceTransition::none()
                };
            }
        }
        MaintenanceAction::ConfirmLockedCacheToml(document) => {
            let parsed = (|| {
                let fields = popup_toml_fields(&document)?;
                let get = |key: &str| {
                    fields
                        .get(key)
                        .cloned()
                        .ok_or_else(|| format!("Missing `{key}`."))
                };
                let snapshot = state
                    .capability
                    .snapshot()
                    .ok_or_else(|| "Locked-cache capability is unavailable.".to_owned())?;
                let mut draft = MaintenanceLockedCacheDraft::from_metadata(&snapshot.metadata)
                    .map_err(str::to_owned)?;
                draft.locked_signatures = get("locked_signatures")?;
                draft.input_cache = get("input_cache")?;
                draft.output_cache = get("output_cache")?;
                draft.filter = get("filter")?;
                draft.request().map_err(str::to_owned)
            })();
            match parsed {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewLockedSignatureCache {
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
                            MaintenanceDialog::LockedCacheToml {
                                editor: PopupEditor::new(document),
                                validation_error: Some(message),
                            },
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        MaintenanceAction::UpdateLockedCacheForm(draft) if draft.is_bounded() => {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(
                    MaintenanceDialog::LockedCacheForm(draft),
                )),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmLockedCacheForm(mut draft) if draft.is_bounded() => {
            match draft.request() {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewLockedSignatureCache {
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
                            MaintenanceDialog::LockedCacheForm(draft),
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
