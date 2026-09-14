//! Maintenance input.
use super::*;

pub fn maintenance_workspace_action(
    view: MaintenanceView,
    row_count: usize,
    key: Input,
) -> Option<Action> {
    let maintenance = |action| Some(Action::Maintenance(action));
    if let Some(delta) = collection_scroll_delta(key) {
        return maintenance(MaintenanceAction::Select { delta, row_count });
    }
    match key {
        Input::Char('[') => maintenance(MaintenanceAction::CycleView { backwards: true }),
        Input::Char(']') | Input::Tab => {
            maintenance(MaintenanceAction::CycleView { backwards: false })
        }
        Input::Up | Input::Char('k') => maintenance(MaintenanceAction::Select {
            delta: -1,
            row_count,
        }),
        Input::Down | Input::Char('j') => maintenance(MaintenanceAction::Select {
            delta: 1,
            row_count,
        }),
        Input::Char('r') => maintenance(MaintenanceAction::InspectCapability),
        Input::Char('x') => maintenance(MaintenanceAction::BeginCancellation),
        Input::Char('o') => maintenance(MaintenanceAction::OpenSelectedEvidence),
        Input::Char('S') => maintenance(MaintenanceAction::OpenSignatures),
        Input::Char('c') if view == MaintenanceView::Sstate => {
            maintenance(MaintenanceAction::OpenReadinessForm)
        }
        Input::Char('d') if view == MaintenanceView::Sstate => {
            maintenance(MaintenanceAction::OpenCleanupForm)
        }
        Input::Char('e') if view == MaintenanceView::Services => maintenance(
            MaintenanceAction::OpenPrServiceForm(yoctui_model::PrServiceOperation::Export),
        ),
        Input::Char('m') if view == MaintenanceView::Services => maintenance(
            MaintenanceAction::OpenPrServiceForm(yoctui_model::PrServiceOperation::Import),
        ),
        Input::Char('l') if view == MaintenanceView::Release => {
            maintenance(MaintenanceAction::OpenLockedCacheForm)
        }
        Input::Char('h') if view == MaintenanceView::Release => {
            maintenance(MaintenanceAction::OpenBuildHistoryForm)
        }
        Input::Char('a') if view == MaintenanceView::Release => {
            maintenance(MaintenanceAction::OpenGitArchiveForm)
        }
        _ => None,
    }
}

pub fn maintenance_dialog_action(dialog: &MaintenanceDialog, key: Input) -> Option<Action> {
    let maintenance = |action| Some(Action::Maintenance(action));
    match dialog {
        MaintenanceDialog::ReadinessToml { editor, .. } => match key {
            Input::Enter => {
                maintenance(MaintenanceAction::ConfirmReadinessToml(editor.text.clone()))
            }
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::CleanupToml { editor, .. } => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmCleanupToml(editor.text.clone())),
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::PrServiceToml {
            operation, editor, ..
        } => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmPrServiceToml {
                operation: *operation,
                document: editor.text.clone(),
            }),
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::LockedCacheToml { editor, .. } => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmLockedCacheToml(
                editor.text.clone(),
            )),
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::BuildHistoryToml { editor, .. } => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmBuildHistoryToml(
                editor.text.clone(),
            )),
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::GitArchiveToml { editor, .. } => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmGitArchiveToml(
                editor.text.clone(),
            )),
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::ReadinessForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Left | Input::Right | Input::Char(' ')
                    if next.field == MaintenanceReadinessField::Mode =>
                {
                    next.mode = match next.mode {
                        yoctui_model::SstateReadinessMode::IsolatedTmpdir => {
                            yoctui_model::SstateReadinessMode::SameTmpdir
                        }
                        yoctui_model::SstateReadinessMode::SameTmpdir => {
                            yoctui_model::SstateReadinessMode::IsolatedTmpdir
                        }
                    };
                }
                Input::Char(character) if !character.is_control() => {
                    let value = match next.field {
                        MaintenanceReadinessField::Targets => &mut next.targets,
                        MaintenanceReadinessField::Output => &mut next.output,
                        MaintenanceReadinessField::Log => &mut next.log,
                        MaintenanceReadinessField::Timeout if character.is_ascii_digit() => {
                            &mut next.timeout
                        }
                        _ => return None,
                    };
                    if value.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        value.push(character);
                    }
                }
                Input::Backspace => match next.field {
                    MaintenanceReadinessField::Targets => {
                        next.targets.pop();
                    }
                    MaintenanceReadinessField::Output => {
                        next.output.pop();
                    }
                    MaintenanceReadinessField::Log => {
                        next.log.pop();
                    }
                    MaintenanceReadinessField::Timeout => {
                        next.timeout.pop();
                    }
                    MaintenanceReadinessField::Mode => return None,
                },
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmReadinessForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateReadinessForm(Box::new(next)))
        }
        MaintenanceDialog::CleanupForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Char(' ') | Input::Left | Input::Right => match next.field {
                    MaintenanceCleanupField::Duplicates => next.duplicates = !next.duplicates,
                    MaintenanceCleanupField::Orphans => next.orphans = !next.orphans,
                    MaintenanceCleanupField::UnreferencedByStamps => {
                        next.unreferenced_by_stamps = !next.unreferenced_by_stamps;
                    }
                    MaintenanceCleanupField::Jobs => return None,
                },
                Input::Char(character)
                    if next.field == MaintenanceCleanupField::Jobs
                        && character.is_ascii_digit()
                        && next.jobs.len() < 5 =>
                {
                    next.jobs.push(character);
                }
                Input::Backspace if next.field == MaintenanceCleanupField::Jobs => {
                    next.jobs.pop();
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmCleanupForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateCleanupForm(Box::new(next)))
        }
        MaintenanceDialog::PrServiceForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Char(character) if !character.is_control() => {
                    if next.file.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        next.file.push(character);
                    }
                }
                Input::Backspace => {
                    next.file.pop();
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmPrServiceForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdatePrServiceForm(Box::new(next)))
        }
        MaintenanceDialog::LockedCacheForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Char(character) if !character.is_control() => {
                    let value = match next.field {
                        MaintenanceLockedCacheField::LockedSignatures => {
                            &mut next.locked_signatures
                        }
                        MaintenanceLockedCacheField::InputCache => &mut next.input_cache,
                        MaintenanceLockedCacheField::OutputCache => &mut next.output_cache,
                        MaintenanceLockedCacheField::Filter => &mut next.filter,
                    };
                    if value.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        value.push(character);
                    }
                }
                Input::Backspace => {
                    match next.field {
                        MaintenanceLockedCacheField::LockedSignatures => {
                            next.locked_signatures.pop();
                        }
                        MaintenanceLockedCacheField::InputCache => {
                            next.input_cache.pop();
                        }
                        MaintenanceLockedCacheField::OutputCache => {
                            next.output_cache.pop();
                        }
                        MaintenanceLockedCacheField::Filter => {
                            next.filter.pop();
                        }
                    };
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmLockedCacheForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateLockedCacheForm(Box::new(next)))
        }
        MaintenanceDialog::BuildHistoryForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Char(' ') | Input::Left | Input::Right if next.field.is_toggle() => {
                    match next.field {
                        MaintenanceBuildHistoryField::ReportVersion => {
                            next.report_version = !next.report_version;
                        }
                        MaintenanceBuildHistoryField::ReportAll => {
                            next.report_all = !next.report_all;
                        }
                        MaintenanceBuildHistoryField::Signatures => {
                            next.signatures = !next.signatures;
                        }
                        MaintenanceBuildHistoryField::SignatureDiff => {
                            next.signature_diff = !next.signature_diff;
                        }
                        MaintenanceBuildHistoryField::NoColour => {
                            next.no_colour = !next.no_colour;
                        }
                        _ => return None,
                    }
                }
                Input::Char(character) if !character.is_control() => {
                    let value = match next.field {
                        MaintenanceBuildHistoryField::FromRevision => &mut next.from_revision,
                        MaintenanceBuildHistoryField::ToRevision => &mut next.to_revision,
                        MaintenanceBuildHistoryField::ExcludePaths => &mut next.exclude_paths,
                        _ => return None,
                    };
                    if value.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        value.push(character);
                    }
                }
                Input::Backspace => {
                    match next.field {
                        MaintenanceBuildHistoryField::FromRevision => {
                            next.from_revision.pop();
                        }
                        MaintenanceBuildHistoryField::ToRevision => {
                            next.to_revision.pop();
                        }
                        MaintenanceBuildHistoryField::ExcludePaths => {
                            next.exclude_paths.pop();
                        }
                        _ => return None,
                    };
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmBuildHistoryForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateBuildHistoryForm(Box::new(next)))
        }
        MaintenanceDialog::GitArchiveForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Char(' ') | Input::Left | Input::Right if next.field.is_toggle() => {
                    match next.field {
                        MaintenanceGitArchiveField::Create => next.create = !next.create,
                        MaintenanceGitArchiveField::Bare => next.bare = !next.bare,
                        MaintenanceGitArchiveField::CreateTag => {
                            next.create_tag = !next.create_tag;
                        }
                        _ => return None,
                    }
                }
                Input::Char(character) if !character.is_control() => {
                    let value = match next.field {
                        MaintenanceGitArchiveField::DataDir => &mut next.data_dir,
                        MaintenanceGitArchiveField::GitDir => &mut next.git_dir,
                        MaintenanceGitArchiveField::BranchName => &mut next.branch_name,
                        MaintenanceGitArchiveField::TagName => &mut next.tag_name,
                        MaintenanceGitArchiveField::CommitSubject => &mut next.commit_subject,
                        MaintenanceGitArchiveField::CommitBody => &mut next.commit_body,
                        MaintenanceGitArchiveField::TagSubject => &mut next.tag_subject,
                        MaintenanceGitArchiveField::TagBody => &mut next.tag_body,
                        MaintenanceGitArchiveField::Exclusions => &mut next.exclusions,
                        MaintenanceGitArchiveField::Notes => &mut next.notes,
                        MaintenanceGitArchiveField::PushRemote => &mut next.push_remote,
                        _ => return None,
                    };
                    if value.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        value.push(character);
                    }
                }
                Input::Backspace => {
                    let value = match next.field {
                        MaintenanceGitArchiveField::DataDir => &mut next.data_dir,
                        MaintenanceGitArchiveField::GitDir => &mut next.git_dir,
                        MaintenanceGitArchiveField::BranchName => &mut next.branch_name,
                        MaintenanceGitArchiveField::TagName => &mut next.tag_name,
                        MaintenanceGitArchiveField::CommitSubject => &mut next.commit_subject,
                        MaintenanceGitArchiveField::CommitBody => &mut next.commit_body,
                        MaintenanceGitArchiveField::TagSubject => &mut next.tag_subject,
                        MaintenanceGitArchiveField::TagBody => &mut next.tag_body,
                        MaintenanceGitArchiveField::Exclusions => &mut next.exclusions,
                        MaintenanceGitArchiveField::Notes => &mut next.notes,
                        MaintenanceGitArchiveField::PushRemote => &mut next.push_remote,
                        _ => return None,
                    };
                    value.pop();
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmGitArchiveForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateGitArchiveForm(Box::new(next)))
        }
        MaintenanceDialog::Confirm(preview) => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmOperation(preview.clone())),
            Input::Esc => maintenance(MaintenanceAction::CancelDialog),
            _ => None,
        },
        MaintenanceDialog::CleanupPhrase { preview, input } => match key {
            Input::Char(character)
                if !character.is_control()
                    && input.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES =>
            {
                let mut next = input.clone();
                next.push(character);
                maintenance(MaintenanceAction::UpdateCleanupPhrase {
                    preview: preview.clone(),
                    input: next,
                })
            }
            Input::Backspace => {
                let mut next = input.clone();
                next.pop();
                maintenance(MaintenanceAction::UpdateCleanupPhrase {
                    preview: preview.clone(),
                    input: next,
                })
            }
            Input::Enter => maintenance(MaintenanceAction::ConfirmCleanupPhrase {
                preview: preview.clone(),
                input: input.clone(),
            }),
            Input::Esc => maintenance(MaintenanceAction::CancelDialog),
            _ => None,
        },
        MaintenanceDialog::ConfirmNetworkPush(preview) => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmNetworkPush(preview.clone())),
            Input::Esc => maintenance(MaintenanceAction::CancelDialog),
            _ => None,
        },
        MaintenanceDialog::ConfirmCancellation(id) => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmCancellation(*id)),
            Input::Esc => maintenance(MaintenanceAction::CancelDialog),
            _ => None,
        },
    }
}
