use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::TerminalCopyViewport => {
            if app.terminal.mode == TerminalWorkbenchMode::Copy
                && let Some(screen) = app.selected_terminal_screen()
                && let Some(row) = screen.rows.get(app.terminal.copy_row)
            {
                let content = row.clone();
                app.terminal.reset_transient_mode();
                return Some(Effect::CopyToClipboard(content));
            }
        }
        Action::TerminalBeginSearch => app.terminal.mode = TerminalWorkbenchMode::Search,
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
