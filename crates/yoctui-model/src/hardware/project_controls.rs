use super::*;

pub(super) fn reduce(app: &mut App, action: HardwareProjectAction) -> Option<Effect> {
    use HardwareProjectAction as A;
    use HardwareProjectOperation as Op;
    let state = &mut app.hardware.projects;
    if state.loading {
        return None;
    }
    let mut request = None;
    match action {
        A::Toggle => {
            state.visible = !state.visible;
            state.error = None;
            state.form = None;
            state.import_browser = None;
            if state.visible {
                request = Some(Op::List);
            }
        }
        A::Select { delta } => {
            if let Some((_, entries, selection)) = &mut state.import_browser {
                *selection = shifted_index(*selection, delta, entries.len());
            } else {
                state.selection = shifted_index(
                    state.selection,
                    delta,
                    if state.project.is_some() {
                        state.entries.len()
                    } else {
                        state.catalog.len()
                    },
                );
            }
        }
        A::Open => {
            if let Some(project) = &state.project {
                let entry = state.entries.get(state.selection)?.clone();
                if entry.is_directory {
                    request = Some(Op::Directory {
                        name: project.name.clone(),
                        relative: state.relative.join(&entry.name),
                    });
                } else if let Some(kind) = HardwareDocumentKind::project_kind(&entry.path) {
                    state.error = None;
                    let root = project.root.clone();
                    let document = HardwareDocument {
                        path: entry.path,
                        category: HardwareCategory::Other,
                        kind,
                    };
                    return viewer::open(app, document, Some(root));
                } else {
                    state.error = Some("Stored only: project viewing supports TXT, PDF, KiCad, Altium and Xpedition schematics.".into());
                }
            } else {
                let project = state.catalog.get(state.selection)?;
                state.project_selection = state.selection;
                request = Some(Op::Directory {
                    name: project.name.clone(),
                    relative: PathBuf::new(),
                });
            }
        }
        A::Parent => {
            if let Some(project) = &state.project {
                request = Some(match state.relative.parent() {
                    Some(parent) => Op::Directory {
                        name: project.name.clone(),
                        relative: parent.to_path_buf(),
                    },
                    None => Op::List,
                });
            }
        }
        A::Reload => {
            request = Some(
                state
                    .project
                    .as_ref()
                    .map_or(Op::List, |project| Op::Directory {
                        name: project.name.clone(),
                        relative: state.relative.clone(),
                    }),
            );
        }
        A::NewName => {
            state.error = None;
            state.form = Some(HardwareProjectForm::Name {
                value: String::new(),
            });
        }
        A::EditName(character) => {
            if let Some(HardwareProjectForm::Name { value }) = &mut state.form
                && !character.is_control()
                && value.len() + character.len_utf8() <= 128
            {
                value.push(character);
            }
        }
        A::Backspace => match &mut state.form {
            Some(HardwareProjectForm::Name { value }) => {
                value.pop();
            }
            Some(HardwareProjectForm::Progress {
                values,
                stage,
                digits,
            }) => {
                digits.pop();
                values[*stage] = digits.parse().unwrap_or(0);
                state.error = None;
            }
            _ => {}
        },
        A::Cancel => {
            state.form = None;
            state.import_browser = None;
            state.error = None;
        }
        A::Confirm => match &state.form {
            Some(HardwareProjectForm::Name { value }) => {
                if let Err(error) = validate_hardware_project_name(value) {
                    state.error = Some(error);
                } else {
                    request = Some(state.project.as_ref().map_or_else(
                        || Op::Create {
                            name: value.clone(),
                        },
                        |project| Op::CreateFolder {
                            name: project.name.clone(),
                            relative: state.relative.clone(),
                            folder: value.clone(),
                        },
                    ));
                }
            }
            Some(HardwareProjectForm::Progress { values, digits, .. }) => {
                if !digits.is_empty() && digits.parse::<u16>().is_ok_and(|number| number > 100) {
                    state.error = Some("Progress must be between 0 and 100.".into());
                } else {
                    request = Some(Op::SaveProgress {
                        name: state.project.as_ref()?.name.clone(),
                        progress: *values,
                    });
                }
            }
            _ => {}
        },
        A::BeginProgress => {
            state.error = None;
            state.form = Some(HardwareProjectForm::Progress {
                values: state.project.as_ref()?.progress,
                stage: 0,
                digits: String::new(),
            });
        }
        A::SelectStage { delta } => {
            if let Some(HardwareProjectForm::Progress { stage, digits, .. }) = &mut state.form {
                *stage = shifted_index(*stage, delta, 6);
                digits.clear();
                state.error = None;
            }
        }
        A::ChangeProgress { delta } => {
            if let Some(HardwareProjectForm::Progress {
                values,
                stage,
                digits,
            }) = &mut state.form
            {
                values[*stage] = (i16::from(values[*stage]) + delta).clamp(0, 100) as u8;
                digits.clear();
                state.error = None;
            }
        }
        A::ProgressDigit(character) => {
            if let Some(HardwareProjectForm::Progress {
                values,
                stage,
                digits,
            }) = &mut state.form
                && character.is_ascii_digit()
                && digits.len() < 3
            {
                digits.push(character);
                let number = digits.parse::<u16>().unwrap_or(101);
                if number <= 100 {
                    values[*stage] = number as u8;
                    state.error = None;
                } else {
                    state.error = Some("Progress must be between 0 and 100.".into());
                }
            }
        }
        A::ToggleProgress => {
            if let Some(HardwareProjectForm::Progress {
                values,
                stage,
                digits,
            }) = &mut state.form
            {
                values[*stage] = if values[*stage] == 100 { 0 } else { 100 };
                digits.clear();
                state.error = None;
            }
        }
        A::BeginImport { directory } => {
            if state.project.is_some() {
                state.import_browser = Some((directory.clone(), Vec::new(), 0));
                request = Some(Op::ImportBrowse { directory });
            }
        }
        A::ImportParent => {
            let (directory, _, _) = state.import_browser.as_ref()?;
            request = Some(Op::ImportBrowse {
                directory: directory.parent()?.to_path_buf(),
            });
        }
        A::ImportOpen => {
            let (_, entries, selection) = state.import_browser.as_ref()?;
            let entry = entries.get(*selection)?;
            request = Some(if entry.is_directory {
                Op::ImportBrowse {
                    directory: entry.path.clone(),
                }
            } else {
                Op::Import {
                    name: state.project.as_ref()?.name.clone(),
                    relative: state.relative.clone(),
                    source: entry.path.clone(),
                }
            });
        }
        A::Request(_) | A::Finished { .. } => unreachable!("worker actions routed separately"),
    }
    request.and_then(|operation| projects::reduce_hardware_project(app, A::Request(operation)))
}
