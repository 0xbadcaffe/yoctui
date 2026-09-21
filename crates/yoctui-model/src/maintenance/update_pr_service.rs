fn update_pr_service(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::OpenPrServiceForm(operation)
            if state.view == MaintenanceView::Services
                && state
                    .capability
                    .snapshot()
                    .is_some_and(|snapshot| snapshot.supports(MaintenanceTool::PrServiceTool)) =>
        {
            if let Some(snapshot) = state.capability.snapshot()
                && let Ok(draft) =
                    MaintenancePrServiceDraft::from_metadata(&snapshot.metadata, operation)
            {
                let mut editor = PopupEditor::new(format!(
                    "# PR service {} request\n# Build directory (read-only): {}\n# Endpoint (read-only): {}\nfile = \"\"\n",
                    match operation {
                        PrServiceOperation::Export => "export",
                        PrServiceOperation::Import => "import",
                    },
                    draft.build_dir.display(),
                    draft.endpoint,
                ));
                let _ = editor.select_toml_value("file");
                return MaintenanceTransition {
                    dialog: MaintenanceDialogUpdate::Open(Box::new(
                        MaintenanceDialog::PrServiceToml {
                            operation,
                            editor,
                            validation_error: None,
                        },
                    )),
                    ..MaintenanceTransition::none()
                };
            }
        }
        MaintenanceAction::ConfirmPrServiceToml {
            operation,
            document,
        } => {
            let parsed = (|| {
                let fields = popup_toml_fields(&document)?;
                let file = fields
                    .get("file")
                    .cloned()
                    .ok_or_else(|| "Missing `file`.".to_owned())?;
                let snapshot = state
                    .capability
                    .snapshot()
                    .ok_or_else(|| "PR service capability is unavailable.".to_owned())?;
                let mut draft =
                    MaintenancePrServiceDraft::from_metadata(&snapshot.metadata, operation)
                        .map_err(str::to_owned)?;
                draft.file = file;
                draft.request().map_err(str::to_owned)
            })();
            match parsed {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewPrService {
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
                            MaintenanceDialog::PrServiceToml {
                                operation,
                                editor: PopupEditor::new(document),
                                validation_error: Some(message),
                            },
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        MaintenanceAction::UpdatePrServiceForm(draft) if draft.is_bounded() => {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::PrServiceForm(
                    draft,
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmPrServiceForm(mut draft) if draft.is_bounded() => {
            match draft.request() {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewPrService {
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
                            MaintenanceDialog::PrServiceForm(draft),
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
