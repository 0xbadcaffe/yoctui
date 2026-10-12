//! Diff controls are scoped before the global F5/logs and Tab/focus bindings.
use crate::{Input, collection_scroll_delta};
use yoctui_model::{Action, App, FileDiffAction as Diff, FocusTarget, Screen};

pub fn file_diff_action(app: &App, input: Input) -> Option<Action> {
    if app.screen != Screen::DiffTool
        || app.focus != FocusTarget::Workspace
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
        || app.keymap_preferences_ui.open
    {
        return None;
    }
    if let Some(delta) = collection_scroll_delta(input) {
        return Some(Action::FileDiff(Diff::Select { delta }));
    }
    let action = match input {
        Input::Tab | Input::BackTab => Diff::SwitchPane,
        Input::Esc => Diff::Back,
        Input::F5 => Diff::Compare,
        Input::Left if app.file_diff.comparison.is_some() => Diff::Horizontal { delta: -8 },
        Input::Right if app.file_diff.comparison.is_some() => Diff::Horizontal { delta: 8 },
        Input::Char(' ') => Diff::Mark,
        Input::Enter => Diff::ToggleDirectory,
        Input::Right => Diff::Expand,
        Input::Left => Diff::Collapse,
        Input::Backspace => Diff::Parent,
        Input::Char('.') => Diff::ToggleHidden,
        Input::Char('r') => Diff::Refresh,
        _ => return None,
    };
    Some(Action::FileDiff(action))
}

pub fn help_workspace_action(app: &App, input: Input) -> Option<Action> {
    if app.screen != Screen::Help
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.onboarding.open
        || app.command_palette_open
        || app.keymap_preferences_ui.open
    {
        return None;
    }
    if let Some(delta) = collection_scroll_delta(input) {
        return Some(Action::ScrollHelp { delta });
    }
    match input {
        Input::Char('g') => Some(Action::OpenOnboarding),
        Input::Esc => Some(Action::Open(Screen::Dashboard)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/file_diff_input.rs"]
mod tests;
