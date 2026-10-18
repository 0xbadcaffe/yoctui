use super::*;

pub(crate) fn reduce(app: &mut App, action: SetupManageAction) -> Option<Effect> {
    use SetupManageAction as A;
    match action {
        A::Fragments | A::Show | A::Enable | A::Disable | A::Custom | A::Status | A::Update => {
            if app.active_dialog().is_some() {
                return None;
            }
            let Some(profile) = profile(app) else {
                app.notification =
                    Some("Select a setup in 2 Setups or configure a build profile first.".into());
                return None;
            };
            let operation = match action {
                A::Fragments => SetupOperation::Fragments,
                A::Show => SetupOperation::Show,
                A::Disable => SetupOperation::Disable,
                A::Status => SetupOperation::Status,
                A::Update => SetupOperation::Update,
                _ => SetupOperation::Enable,
            };
            let fragment = if matches!(action, A::Show | A::Enable | A::Disable) {
                let Some(entry) = app
                    .bitbake_setup
                    .management
                    .fragments
                    .get(app.bitbake_setup.management.selected)
                else {
                    return None;
                };
                entry
                    .name
                    .strip_suffix("...")
                    .unwrap_or(&entry.name)
                    .to_owned()
            } else {
                String::new()
            };
            let dialog = SetupOperationDialog {
                operation,
                profile,
                tool: app.bitbake_setup.tool.clone(),
                fragment,
                editor: None,
                config_policy: 0,
                backup_conflicts: false,
                offline: false,
                preparing: None,
                error: setup_operation_blocked(app),
            };
            open_dialog(app, Dialog::BitBakeSetupOperation(Box::new(dialog)));
        }
        A::Refresh => {
            if let Some(profile) = app.bitbake_setup.management.profile.clone() {
                return inspect(app, profile);
            }
        }
        A::Completed(profile) => {
            if app.bitbake_setup.management.profile.as_ref() == Some(&profile) {
                return inspect(app, profile);
            }
            app.notification=Some("Setup operation completed. Refresh/reverify the selected environment before building.".into());
        }
        A::Select(delta) => {
            let state = &mut app.bitbake_setup.management;
            if let Some(lines) = &state.output {
                state.output_scroll = state
                    .output_scroll
                    .saturating_add_signed(delta)
                    .min(lines.len().saturating_sub(1));
                return None;
            }
            state.selected = state
                .selected
                .saturating_add_signed(delta)
                .min(state.fragments.len().saturating_sub(1));
        }
        A::Loaded { generation, result } => {
            let state = &mut app.bitbake_setup.management;
            if state.generation != generation {
                return None;
            }
            state.loading = false;
            match result {
                Ok(mut fragments) => {
                    let name = state.fragments.get(state.selected).map(|f| f.name.clone());
                    fragments.truncate(2048);
                    state.selected = name
                        .and_then(|n| fragments.iter().position(|f| f.name == n))
                        .unwrap_or(0);
                    state.fragments = fragments;
                    state.error = None;
                }
                Err(e) => state.error = Some(e),
            }
        }
        A::Read { generation, result } => {
            let state = &mut app.bitbake_setup.management;
            if state.generation != generation {
                return None;
            }
            state.loading = false;
            match result {
                Ok(text) => state.output = Some(text.lines().map(str::to_owned).collect()),
                Err(e) => state.error = Some(e),
            }
        }
        A::Confirm => {
            let blocked = setup_operation_blocked(app);
            let Some(Dialog::BitBakeSetupOperation(dialog)) = app.active_dialog_mut() else {
                return None;
            };
            if dialog.preparing.is_some() || dialog.editor.is_some() {
                return None;
            }
            if let Some(error) = blocked {
                dialog.error = Some(error);
                return None;
            }
            if matches!(
                dialog.operation,
                SetupOperation::Enable | SetupOperation::Disable | SetupOperation::Show
            ) && !valid_setup_fragment(&dialog.fragment)
            {
                dialog.error=Some("Use a complete fragment ID, e.g. machine/qemux86-64. Prefix-wide disabling and path traversal are not allowed.".into());
                return None;
            }
            let dialog = (**dialog).clone();
            let state = &mut app.bitbake_setup.management;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            if let Some(Dialog::BitBakeSetupOperation(d)) = app.active_dialog_mut() {
                d.preparing = Some(generation);
                d.error = None;
            }
            return Some(Effect::BitBakeSetup(BitBakeSetupRequest::Manage(
                SetupManageRequest::Prepare { generation, dialog },
            )));
        }
        A::Prepared { generation, result } => {
            if !matches!(app.active_dialog(),Some(Dialog::BitBakeSetupOperation(d)) if d.preparing==Some(generation))
            {
                return None;
            }
            let result = if let Some(error) = setup_operation_blocked(app) {
                Err(error)
            } else {
                result
            };
            match result {
                Err(error) => {
                    if let Some(Dialog::BitBakeSetupOperation(d)) = app.active_dialog_mut() {
                        d.preparing = None;
                        d.error = Some(error);
                    }
                }
                Ok(request) => {
                    let Some(Dialog::BitBakeSetupOperation(dialog)) = app.active_dialog().cloned()
                    else {
                        return None;
                    };
                    close_dialog(app);
                    if matches!(
                        dialog.operation,
                        SetupOperation::Fragments
                            | SetupOperation::Show
                            | SetupOperation::Enable
                            | SetupOperation::Disable
                    ) {
                        app.bitbake_setup.management.trusted_profile = Some(dialog.profile.clone());
                    }
                    if dialog.operation == SetupOperation::Fragments {
                        let state = &mut app.bitbake_setup.management;
                        state.output = None;
                        if state.profile.as_ref() != Some(&dialog.profile) {
                            state.fragments.clear();
                            state.selected = 0;
                        }
                        state.visible = true;
                        return inspect(app, dialog.profile);
                    }
                    if matches!(
                        dialog.operation,
                        SetupOperation::Status | SetupOperation::Show
                    ) {
                        let state = &mut app.bitbake_setup.management;
                        if state.profile.as_ref() != Some(&dialog.profile) {
                            state.fragments.clear();
                            state.selected = 0;
                        }
                        state.visible = true;
                        state.profile = Some(dialog.profile.clone());
                        state.loading = true;
                        state.error = None;
                        state.output = Some(Vec::new());
                        state.output_title = dialog.operation.label().into();
                        state.output_scroll = 0;
                        return Some(Effect::BitBakeSetup(BitBakeSetupRequest::Manage(
                            SetupManageRequest::Read {
                                generation,
                                dialog: *dialog,
                            },
                        )));
                    }
                    app.screen = Screen::TerminalSessions;
                    app.bitbake_setup.management.error=Some("Operation launched; displayed fragments are last inspected state until refresh succeeds. Updates are not rolled back automatically.".into());
                    app.focus = FocusTarget::Workspace;
                    app.focus_return = None;
                    app.prepare_created_terminal_selection();
                    return Some(Effect::Terminal(TerminalEffect::Create {
                        name: request.name,
                        kind: request.kind,
                        cwd: request.cwd,
                        program: request.program,
                        arguments: request.arguments,
                        completion: request.completion,
                    }));
                }
            }
        }
        A::Cancel => {
            if let Some(Dialog::BitBakeSetupOperation(d)) = app.active_dialog_mut() {
                if d.editor.take().is_none() {
                    close_dialog(app);
                }
            } else {
                let state = &mut app.bitbake_setup.management;
                state.generation = state.generation.wrapping_add(1);
                state.loading = false;
                if state.output.take().is_none() {
                    state.visible = false;
                }
            }
            return Some(Effect::BitBakeSetup(BitBakeSetupRequest::CancelManagement));
        }
        _ => {
            let Some(Dialog::BitBakeSetupOperation(d)) = app.active_dialog_mut() else {
                return None;
            };
            if d.preparing.is_some() {
                return None;
            }
            match action {
                A::Policy if d.operation == SetupOperation::Update => {
                    d.config_policy = (d.config_policy + 1) % 3
                }
                A::Backup if d.operation == SetupOperation::Update => {
                    d.backup_conflicts = !d.backup_conflicts
                }
                A::Offline
                    if matches!(d.operation, SetupOperation::Status | SetupOperation::Update) =>
                {
                    d.offline = !d.offline
                }
                A::Edit
                    if matches!(
                        d.operation,
                        SetupOperation::Enable | SetupOperation::Disable | SetupOperation::Show
                    ) =>
                {
                    let mut e = TextAreaState::new(d.fragment.clone());
                    e.selection = Some((0, e.text.len()));
                    d.editor = Some(e);
                }
                A::Insert(text) => {
                    if let Some(e) = &mut d.editor {
                        let replace = e.selection.map_or(0, |(a, b)| b.saturating_sub(a));
                        if !text.chars().any(char::is_control)
                            && e.text.len().saturating_sub(replace) + text.len() <= 512
                        {
                            e.insert(&text);
                            d.error = None;
                        } else {
                            d.error = Some(
                                "Fragment IDs must be single-line text up to 512 bytes.".into(),
                            );
                        }
                    }
                }
                A::Motion(m) => {
                    if let Some(e) = &mut d.editor {
                        e.move_cursor(m);
                    }
                }
                A::Backspace => {
                    if let Some(e) = &mut d.editor {
                        e.backspace();
                    }
                }
                A::Clear => {
                    if let Some(e) = &mut d.editor {
                        *e = TextAreaState::new(String::new());
                    }
                }
                A::AcceptEdit => {
                    if let Some(e) = d.editor.take() {
                        d.fragment = e.text;
                    }
                }
                _ => {}
            }
        }
    }
    None
}
