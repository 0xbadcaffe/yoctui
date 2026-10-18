//! Modal input owns every key; field text can never become a global shortcut.
use crate::Input;
use yoctui_model::{
    Action, App, BitBakeSetupAction as A, Dialog, FocusTarget, Screen, TextAreaMotion,
};

pub fn bitbake_setup_action(app: &App, key: Input) -> Option<Action> {
    let action = if let Some(Dialog::BitBakeSetup(wizard)) = app.active_dialog() {
        if key == Input::Esc {
            A::Cancel
        } else if wizard.preparing.is_some() {
            return None;
        } else if wizard.editor.is_some() {
            match key {
                Input::Char(c) => A::Insert(c.to_string()),
                Input::Backspace => A::Backspace,
                Input::CtrlU => A::Clear,
                Input::Left => A::Motion(TextAreaMotion::Left),
                Input::Right => A::Motion(TextAreaMotion::Right),
                Input::Home => A::Motion(TextAreaMotion::LineStart),
                Input::End => A::Motion(TextAreaMotion::LineEnd),
                Input::Enter => A::AcceptEdit,
                _ => return None,
            }
        } else if wizard.review {
            if key == Input::Enter {
                A::Confirm
            } else {
                return None;
            }
        } else {
            match key {
                Input::Up | Input::BackTab => A::Field(-1),
                Input::Down | Input::Tab => A::Field(1),
                Input::Enter | Input::Char('e') => A::Edit,
                Input::Char('s') | Input::CtrlS => A::Review,
                _ => return None,
            }
        }
    } else {
        if app.active_dialog().is_some()
            || app.screen != Screen::BuildEnvironment
            || app.focus != FocusTarget::Workspace
            || app
                .build_environment_draft
                .as_ref()
                .is_some_and(|d| d.editing)
        {
            return None;
        }
        match key {
            Input::Char('1') => A::Environment,
            Input::Char('2') => A::Catalogue,
            Input::Char('w') => A::New,
            _ if !app.bitbake_setup.catalogue => return None,
            Input::Char('n') => A::New,
            Input::Char('o') => A::Root,
            Input::Char('r') => A::Refresh,
            Input::Up | Input::Char('k') => A::Select(-1),
            Input::Down | Input::Char('j') => A::Select(1),
            Input::PageUp => A::Select(-10),
            Input::PageDown => A::Select(10),
            Input::Home => A::Select(isize::MIN),
            Input::End => A::Select(isize::MAX),
            Input::Enter => A::Import,
            Input::Esc => A::Environment,
            _ => return None,
        }
    };
    Some(Action::BitBakeSetup(action))
}

#[cfg(test)]
#[path = "tests/bitbake_setup.rs"]
mod tests;
