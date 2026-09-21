fn update_git_archive(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        MaintenanceAction::OpenGitArchiveForm
            if state.view == MaintenanceView::Release
                && state
                    .capability
                    .snapshot()
                    .is_some_and(|snapshot| snapshot.supports(MaintenanceTool::GitArchive)) =>
        {
            let draft = MaintenanceGitArchiveDraft::default();
            let mut editor = PopupEditor::new(format!(
                "# Git release archive request\ndata_dir = \"{}\"\ngit_dir = \"{}\"\ncreate = {}\nbare = {}\ncreate_tag = {}\nbranch_name = \"{}\"\ntag_name = \"{}\"\ncommit_subject = \"{}\"\ncommit_body = \"{}\"\ntag_subject = \"{}\"\ntag_body = \"{}\"\nexclusions = \"{}\"\nnotes = \"{}\"\npush_remote = \"{}\"\n",
                draft.data_dir,
                draft.git_dir,
                draft.create,
                draft.bare,
                draft.create_tag,
                draft.branch_name,
                draft.tag_name,
                draft.commit_subject,
                draft.commit_body,
                draft.tag_subject,
                draft.tag_body,
                draft.exclusions,
                draft.notes,
                draft.push_remote
            ));
            let _ = editor.select_toml_value("data_dir");
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(
                    MaintenanceDialog::GitArchiveToml {
                        editor,
                        validation_error: None,
                    },
                )),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmGitArchiveToml(document) => {
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
                let draft = MaintenanceGitArchiveDraft {
                    field: MaintenanceGitArchiveField::DataDir,
                    data_dir: get("data_dir")?,
                    git_dir: get("git_dir")?,
                    create: boolean("create")?,
                    bare: boolean("bare")?,
                    create_tag: boolean("create_tag")?,
                    branch_name: get("branch_name")?,
                    tag_name: get("tag_name")?,
                    commit_subject: get("commit_subject")?,
                    commit_body: get("commit_body")?,
                    tag_subject: get("tag_subject")?,
                    tag_body: get("tag_body")?,
                    exclusions: get("exclusions")?,
                    notes: get("notes")?,
                    push_remote: get("push_remote")?,
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
                        effect: Some(MaintenanceEffect::PreviewGitArchive {
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
                            MaintenanceDialog::GitArchiveToml {
                                editor: PopupEditor::new(document),
                                validation_error: Some(message),
                            },
                        )),
                        ..MaintenanceTransition::none()
                    };
                }
            }
        }
        MaintenanceAction::UpdateGitArchiveForm(draft) if draft.is_bounded() => {
            return MaintenanceTransition {
                dialog: MaintenanceDialogUpdate::Open(Box::new(MaintenanceDialog::GitArchiveForm(
                    draft,
                ))),
                ..MaintenanceTransition::none()
            };
        }
        MaintenanceAction::ConfirmGitArchiveForm(mut draft) if draft.is_bounded() => {
            match draft.request() {
                Ok(request) => {
                    let Some(capability_request) = state.capability.request() else {
                        return MaintenanceTransition::none();
                    };
                    return MaintenanceTransition {
                        effect: Some(MaintenanceEffect::PreviewGitArchive {
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
                            MaintenanceDialog::GitArchiveForm(draft),
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
