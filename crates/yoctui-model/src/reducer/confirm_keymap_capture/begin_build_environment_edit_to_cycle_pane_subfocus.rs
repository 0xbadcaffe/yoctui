use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::BeginBuildEnvironmentEdit => {
            let profile = match &app.build_environment {
                BuildEnvironmentState::Configured(profile)
                | BuildEnvironmentState::Connected(profile)
                | BuildEnvironmentState::Failed { profile, .. }
                | BuildEnvironmentState::Verifying { profile, .. } => Some(profile),
                BuildEnvironmentState::Unconfigured => None,
            };
            app.build_environment_draft = Some(BuildEnvironmentDraft {
                source: profile.map_or_else(String::new, |p| p.source_dir.display().to_string()),
                build: profile.map_or_else(String::new, |p| p.build_dir.display().to_string()),
                script: profile.map_or_else(String::new, |p| p.init_script.display().to_string()),
                field: BuildEnvironmentField::Source,
                editing: true,
            });
        }
        Action::SelectBuildEnvironmentField { delta } => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let index: usize = match draft.field {
                    BuildEnvironmentField::Source => 0,
                    BuildEnvironmentField::Build => 1,
                    BuildEnvironmentField::Script => 2,
                };
                let next = if delta.is_negative() {
                    index.saturating_sub(delta.unsigned_abs())
                } else {
                    index.saturating_add(delta as usize).min(2)
                };
                draft.field = match next {
                    0 => BuildEnvironmentField::Source,
                    1 => BuildEnvironmentField::Build,
                    _ => BuildEnvironmentField::Script,
                };
            }
        }
        Action::AppendBuildEnvironmentField(character) => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let value = match draft.field {
                    BuildEnvironmentField::Source => &mut draft.source,
                    BuildEnvironmentField::Build => &mut draft.build,
                    BuildEnvironmentField::Script => &mut draft.script,
                };
                value.push(character);
            }
        }
        Action::BackspaceBuildEnvironmentField => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let value = match draft.field {
                    BuildEnvironmentField::Source => &mut draft.source,
                    BuildEnvironmentField::Build => &mut draft.build,
                    BuildEnvironmentField::Script => &mut draft.script,
                };
                value.pop();
            }
        }
        Action::FinishBuildEnvironmentEdit => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                draft.editing = false;
            }
        }
        Action::CancelBuildEnvironmentEdit => app.build_environment_draft = None,
        Action::ApplyBuildEnvironmentProfile => {
            if let Some(draft) = app.build_environment_draft.take() {
                let profile = BuildEnvironmentProfile {
                    source_dir: PathBuf::from(draft.source),
                    build_dir: PathBuf::from(draft.build),
                    init_script: PathBuf::from(draft.script),
                };
                let _ = update(app, Action::ConfigureBuildEnvironment(profile));
            }
        }
        Action::BeginBuildEnvironmentVerification => {
            let profile = match &app.build_environment {
                BuildEnvironmentState::Configured(profile)
                | BuildEnvironmentState::Failed { profile, .. } => profile.clone(),
                BuildEnvironmentState::Verifying { .. } => return None,
                BuildEnvironmentState::Unconfigured | BuildEnvironmentState::Connected(_) => {
                    app.notification =
                        Some("Select a build environment before verification.".into());
                    return None;
                }
            };
            app.build_environment_generation = app.build_environment_generation.wrapping_add(1);
            let generation = app.build_environment_generation;
            app.build_environment = BuildEnvironmentState::Verifying {
                profile: profile.clone(),
                generation,
            };
            return Some(Effect::VerifyBuildEnvironment {
                profile,
                generation,
            });
        }
        Action::BuildEnvironmentVerified { generation } => {
            if let BuildEnvironmentState::Verifying {
                profile,
                generation: pending,
            } = &app.build_environment
                && *pending == generation
            {
                app.build_environment = BuildEnvironmentState::Connected(profile.clone());
                app.notification = None;
            }
        }
        Action::BuildEnvironmentVerificationFailed {
            generation,
            message,
        } => {
            if let BuildEnvironmentState::Verifying {
                profile,
                generation: pending,
            } = &app.build_environment
                && *pending == generation
            {
                app.build_environment = BuildEnvironmentState::Failed {
                    profile: profile.clone(),
                    message: message.clone(),
                };
                app.notification = Some(format!("BitBake connection failed: {message}"));
            }
        }
        Action::CycleFocus { backwards } => {
            if matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette) {
                return None;
            }
            let targets = pane_focus_targets(app).collect::<Vec<_>>();
            let current = targets
                .iter()
                .position(|target| *target == app.focus)
                .unwrap_or(0);
            let next = if backwards {
                (current + targets.len() - 1) % targets.len()
            } else {
                (current + 1) % targets.len()
            };
            app.focus = targets[next];
            if app.zoomed_pane.is_some() {
                app.zoomed_pane = Some(app.focus);
            }
        }
        Action::CyclePaneSubfocus { backwards } => match app.focus {
            FocusTarget::Workspace => {
                let count = workspace_subfocus_count(app.screen);
                let current = match app.workspace_subfocus {
                    WorkspaceSubfocus::Main => 0,
                    WorkspaceSubfocus::Secondary => 1,
                    WorkspaceSubfocus::Context => 2,
                }
                .min(count.saturating_sub(1));
                let next = if backwards {
                    (current + count - 1) % count
                } else {
                    (current + 1) % count
                };
                app.workspace_subfocus = match next {
                    0 => WorkspaceSubfocus::Main,
                    1 => WorkspaceSubfocus::Secondary,
                    _ => WorkspaceSubfocus::Context,
                };
            }
            FocusTarget::Inspector => {
                let current = match app.inspector_subfocus {
                    InspectorSubfocus::Facts => 0,
                    InspectorSubfocus::Output => 1,
                    InspectorSubfocus::Actions => 2,
                };
                let next = if backwards {
                    (current + 2) % 3
                } else {
                    (current + 1) % 3
                };
                app.inspector_subfocus = match next {
                    0 => InspectorSubfocus::Facts,
                    1 => InspectorSubfocus::Output,
                    _ => InspectorSubfocus::Actions,
                };
            }
            FocusTarget::Navigator | FocusTarget::Dialog | FocusTarget::CommandPalette => {}
        },
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
