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

impl InteractiveRuntime {
    pub(super) async fn route_terminal_workspace(
        &mut self,
        input: Input,
        replayed_context_action: bool,
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
        } else if runtime.app.selected_terminal_is_writer() {
            if let (Some(bytes), Some(session), Some(details)) = (
                terminal_input_bytes_for_app(&runtime.app, input),
                runtime.app.selected_terminal_session(),
                runtime.app.selected_terminal_details(),
            ) {
                let effect = Effect::Terminal(yoctui_model::TerminalEffect::Input {
                    session_id: session.id,
                    writer_epoch: details.writer_epoch,
                    bytes,
                });
                let _ =
                    submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect);
            }
        } else {
            runtime.app.notification =
                Some("Terminal is read-only; press o or Ctrl+B o to take writer control.".into());
        }
    }
}
