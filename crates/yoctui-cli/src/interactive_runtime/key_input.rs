use super::*;

pub(super) enum KeyRouteOutcome {
    Handled,
    ContinueLoop,
}

impl KeyRouteOutcome {
    fn continues_loop(self) -> bool {
        matches!(self, Self::ContinueLoop)
    }
}

impl InteractiveRuntime {
    pub(super) async fn handle_key(&mut self, k: crossterm::event::KeyEvent) -> Result<bool> {
        let runtime = self;
        let Some(mut input) = input_from_key(k) else {
            return Ok(true);
        };
        if input == Input::Esc
            && runtime.app.screen == Screen::BuildEnvironment
            && runtime.app.active_dialog().is_none()
            && !runtime.app.menu.is_open()
            && runtime.clone_operation.is_some()
        {
            clone_operation::cancel(&mut runtime.app, &mut runtime.clone_operation);
            runtime.render_scheduler.invalidate(RenderCause::State);
            return Ok(true);
        }
        if let Some(Dialog::EnvironmentSetup(setup)) = runtime.app.active_dialog() {
            if let Some(action) = yoctui_app::environment_setup_action(setup, input)
                && let Some(effect) = compatibility_workspace_action(&mut runtime.app, action)
            {
                runtime.environment_browser_io.submit(effect);
            }
            return Ok(true);
        }
        if runtime.app.active_dialog().is_none()
            && !runtime.app.menu.is_open()
            && !runtime.app.onboarding.open
        {
            match runtime.prefix_state.feed(input, Instant::now()) {
                PrefixEvent::Awaiting => {
                    runtime.app.notification = Some(
                        "Prefix Ctrl+B: t terminals | c create | n/p session | %/\" split | z zoom | [ copy | / search | d detach | : palette | ? help"
                            .into(),
                    );
                    return Ok(true);
                }
                PrefixEvent::Command(command) => {
                    if matches!(
                        command,
                        PrefixCommand::CreateSession | PrefixCommand::TakeControl
                    ) {
                        if let Some(daemon_client) = runtime.daemon_runtime.as_mut() {
                            if let Err(error) = daemon_client.route_prefix(&runtime.app, command) {
                                runtime.app.notification =
                                    Some(format!("Prefix command failed: {error}"));
                            }
                        } else {
                            runtime.app.notification =
                                Some("Daemon is unavailable for terminal sessions.".into());
                        }
                    }
                    if command == PrefixCommand::Detach
                        && let Some(daemon_client) = runtime.daemon_runtime.as_mut()
                        && let Err(error) = daemon_client.detach_terminal(&runtime.app)
                    {
                        runtime.app.notification = Some(format!("Terminal detach failed: {error}"));
                    }
                    if command == PrefixCommand::OpenTerminalSessions {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::Open(Screen::TerminalSessions),
                        );
                    }
                    let terminal_action = match command {
                        PrefixCommand::CopyMode => Some(Action::TerminalEnterCopyMode),
                        PrefixCommand::Search => Some(Action::TerminalBeginSearch),
                        PrefixCommand::Rename => Some(Action::TerminalBeginRename),
                        PrefixCommand::ReleaseControl => Some(Action::TerminalReleaseControl),
                        PrefixCommand::Kill => Some(Action::TerminalBeginKill),
                        PrefixCommand::Zoom => Some(Action::TogglePaneZoom),
                        _ => None,
                    };
                    if let Some(action) = terminal_action
                        && let Some(effect @ Effect::Terminal(_)) =
                            compatibility_workspace_action(&mut runtime.app, action)
                    {
                        let _ = submit_daemon_effect(
                            &mut runtime.daemon_runtime,
                            &mut runtime.app,
                            &effect,
                        );
                    }
                    if command == PrefixCommand::NextSession {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::SelectPtySession { delta: 1 },
                        );
                    } else if command == PrefixCommand::PreviousSession {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::SelectPtySession { delta: -1 },
                        );
                    }
                    if command == PrefixCommand::SplitHorizontal
                        || command == PrefixCommand::SplitVertical
                    {
                        let axis = if command == PrefixCommand::SplitHorizontal {
                            yoctui_model::SplitAxis::Horizontal
                        } else {
                            yoctui_model::SplitAxis::Vertical
                        };
                        match runtime
                            .app
                            .pane_layout
                            .split(runtime.app.pane_layout.focused, axis)
                        {
                            Ok(_) => runtime.app.notification = Some("Terminal pane split".into()),
                            Err(error) => {
                                runtime.app.notification =
                                    Some(format!("Pane split failed: {error}"))
                            }
                        }
                    }
                    if command == PrefixCommand::ClosePane {
                        match runtime
                            .app
                            .pane_layout
                            .close(runtime.app.pane_layout.focused)
                        {
                            Ok(_) => runtime.app.notification = Some("Terminal pane closed".into()),
                            Err(error) => {
                                runtime.app.notification =
                                    Some(format!("Pane close failed: {error}"))
                            }
                        }
                    }
                    runtime.app.notification = Some(match command {
                        PrefixCommand::CommandPalette => {
                            let _ = compatibility_workspace_action(
                                &mut runtime.app,
                                Action::OpenCommandPalette,
                            );
                            "Command palette opened".into()
                        }
                        PrefixCommand::Help => {
                            let action = if runtime.app.screen == Screen::TerminalSessions
                                || runtime.app.platform_menuconfig_visible()
                            {
                                Action::TerminalToggleHelp
                            } else {
                                Action::Open(Screen::Help)
                            };
                            let _ = compatibility_workspace_action(&mut runtime.app, action);
                            "Help opened".into()
                        }
                        PrefixCommand::CreateSession => "Create terminal session requested".into(),
                        PrefixCommand::NextSession => "Next terminal session".into(),
                        PrefixCommand::PreviousSession => "Previous terminal session".into(),
                        PrefixCommand::SplitHorizontal => "Horizontal split requested".into(),
                        PrefixCommand::SplitVertical => "Vertical split requested".into(),
                        PrefixCommand::ClosePane => "Terminal pane close requested".into(),
                        PrefixCommand::Detach => "Detached from terminal session".into(),
                        PrefixCommand::TakeControl => "PTY writer control requested".into(),
                        PrefixCommand::OpenTerminalSessions => "Terminal Sessions opened".into(),
                        PrefixCommand::CopyMode => "Terminal copy mode opened".into(),
                        PrefixCommand::Search => "Terminal search opened".into(),
                        PrefixCommand::Rename => "Terminal rename opened".into(),
                        PrefixCommand::ReleaseControl => "Terminal writer release requested".into(),
                        PrefixCommand::Kill => "Terminal kill confirmation opened".into(),
                        PrefixCommand::Zoom => "Terminal pane zoom toggled".into(),
                    });
                    return Ok(true);
                }
                PrefixEvent::Literal(next) => input = next,
            }
        }
        let mut replayed_context_action = false;
        if runtime.app.menu.is_open() {
            match menu_action(&runtime.app, input) {
                Some(MenuInputResult::Reduce(action)) => {
                    let _ = compatibility_workspace_action(&mut runtime.app, *action);
                    return Ok(true);
                }
                Some(MenuInputResult::ActivateCommand(command)) => {
                    let _ = compatibility_workspace_action(&mut runtime.app, Action::CloseMenu);
                    let action = yoctui_model::command_action(&runtime.app, command);
                    let _ = compatibility_workspace_action(&mut runtime.app, action);
                    return Ok(true);
                }
                Some(MenuInputResult::ActivateContext(replay)) => {
                    let _ = compatibility_workspace_action(&mut runtime.app, Action::CloseMenu);
                    input = replay;
                    replayed_context_action = true;
                }
                Some(MenuInputResult::ActivateDisabled(reason)) => {
                    let _ = compatibility_workspace_action(&mut runtime.app, Action::CloseMenu);
                    runtime.app.notification = Some(reason);
                    return Ok(true);
                }
                None => return Ok(true),
            }
        }
        if runtime.app.onboarding.open {
            let effect = onboarding_action(&runtime.app, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            match effect {
                Some(Effect::PersistOnboarding) => {
                    let result = persist_onboarding(
                        runtime.session_path.as_deref(),
                        &mut runtime.session,
                        &runtime.app,
                    );
                    let action = match result {
                        Ok(()) => Action::OnboardingPersisted,
                        Err(error) => Action::OnboardingPersistenceFailed(error.to_string()),
                    };
                    let _ = compatibility_workspace_action(&mut runtime.app, action);
                }
                Some(effect @ Effect::GetImageArtifacts(_)) => {
                    begin_image_artifact_operation(
                        &mut runtime.app,
                        runtime.image_artifact_adapter.as_ref(),
                        &mut runtime.image_artifact_operation,
                        effect,
                    );
                }
                _ => {}
            }
            return Ok(true);
        }
        if let Some(action) = yoctui_app::saved_build_workspace_action(&runtime.app, input) {
            let _ = compatibility_workspace_action(&mut runtime.app, action);
            return Ok(true);
        }
        if let Some(action) = notification_popup_action(&runtime.app, input) {
            let _ = compatibility_workspace_action(&mut runtime.app, action);
            return Ok(true);
        }
        if let Some(action) =
            direct_menu_shortcut_action(&runtime.app, input, replayed_context_action)
        {
            let _ = compatibility_workspace_action(&mut runtime.app, action);
            return Ok(true);
        }
        if let Some(outcome) = runtime.route_command_and_admin_dialogs(input).await? {
            return Ok(outcome.continues_loop());
        }
        if let Some(outcome) = runtime.route_sdk_test_wic_dialogs(input).await? {
            return Ok(outcome.continues_loop());
        }
        if let Some(outcome) = runtime.route_image_and_editor_dialogs(input).await? {
            return Ok(outcome.continues_loop());
        }
        if let Some(outcome) = runtime.route_devtool_and_build_dialogs(input).await? {
            return Ok(outcome.continues_loop());
        }
        if let Some(outcome) = runtime
            .route_terminal_and_primary_workspaces(input, replayed_context_action)
            .await?
        {
            return Ok(outcome.continues_loop());
        }
        if let Some(outcome) = runtime.route_recipe_and_layer_workspaces(input).await? {
            return Ok(outcome.continues_loop());
        }
        if let Some(outcome) = runtime.route_remaining_workspaces(input).await? {
            return Ok(outcome.continues_loop());
        }
        Ok(false)
    }
}
