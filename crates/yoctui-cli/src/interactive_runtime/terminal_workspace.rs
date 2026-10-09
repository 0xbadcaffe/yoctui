use super::*;

/// Outer Some traps every key, including keys without a supported Input mapping.
pub(crate) fn terminal_kill_review_key(
    app: &App,
    key: crossterm::event::KeyEvent,
) -> Option<Option<Action>> {
    (app.terminal.mode == yoctui_model::TerminalWorkbenchMode::KillConfirmation).then(|| {
        input_from_key(key).and_then(|input| yoctui_app::terminal_workspace_action(app, input))
    })
}

pub(crate) fn terminal_writer_key_effect(
    app: &App,
    key: crossterm::event::KeyEvent,
) -> Option<Effect> {
    if app.screen != Screen::TerminalSessions
        || app.focus != yoctui_model::FocusTarget::Workspace
        || app.terminal.mode != yoctui_model::TerminalWorkbenchMode::Live
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
        || app.keymap_preferences_ui.open
        || !app.selected_terminal_is_writer()
    {
        return None;
    }
    let details = app.selected_terminal_details()?;
    let application_cursor = app
        .selected_terminal_screen()
        .is_some_and(|screen| screen.application_cursor);
    Some(Effect::Terminal(yoctui_model::TerminalEffect::Input {
        session_id: details.id,
        writer_epoch: details.writer_epoch,
        bytes: terminal_key_bytes(key, application_cursor)?,
    }))
}

impl InteractiveRuntime {
    pub(super) async fn route_terminal_workspace(
        &mut self,
        input: Input,
        replayed_context_action: bool,
        key: crossterm::event::KeyEvent,
    ) {
        let runtime = self;
        let terminal_action = if replayed_context_action {
            yoctui_app::terminal_context_action(input)
        } else {
            yoctui_app::terminal_workspace_action(&runtime.app, input)
        };
        if let Some(action) = terminal_action {
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(effect @ Effect::Terminal(_)) => {
                    let _ = submit_daemon_effect(
                        &mut runtime.daemon_runtime,
                        &mut runtime.app,
                        &effect,
                    );
                }
                Some(Effect::CopyToClipboard(content)) => {
                    copy_to_clipboard(&mut runtime.app, content).await;
                }
                _ => {}
            }
        } else if runtime.app.terminal.mode == yoctui_model::TerminalWorkbenchMode::KillConfirmation
        {
            // Unmapped review input must never reach the current writer.
        } else if let Some(effect) = terminal_writer_key_effect(&runtime.app, key) {
            let _ = submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect);
        } else if runtime.app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Live
            && !runtime.app.selected_terminal_is_writer()
        {
            runtime.app.notification = Some(match runtime.app.selected_terminal_session() {
                Some(session) if session.lifecycle != yoctui_model::ClientDaemonLifecycle::Running =>
                    "This terminal has exited; its history is read-only. Create a new shell with Ctrl+B c, or select a running session.".into(),
                Some(_) if runtime.app.selected_terminal_details().is_some_and(|details| details.writer.is_some()) =>
                    "Another client owns this terminal. Release control in that client before taking it here.".into(),
                Some(_) => "Press o or Ctrl+B o to take writer control of this running terminal.".into(),
                None => "Select a running terminal, or create a new shell with Ctrl+B c.".into(),
            });
        }
    }
}
