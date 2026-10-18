//! Modal input owns every key; field text can never become a global shortcut.
use crate::Input;
use yoctui_model::{
    Action, App, BitBakeSetupAction as A, Dialog, FocusTarget, Screen, TextAreaMotion,
};

pub fn bitbake_setup_action(app: &App, key: Input) -> Option<Action> {
    if matches!(app.active_dialog(), Some(Dialog::BitBakeSetupOperation(_))) {
        return management_action(app, key);
    }
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
            Input::Char('3') => A::Manage(yoctui_model::SetupManageAction::Fragments),
            _ if app.bitbake_setup.management.visible => return management_action(app, key),
            _ if !app.bitbake_setup.catalogue => return None,
            Input::Char('n') => A::New,
            Input::Char('o') => A::Root,
            Input::Char('r') => A::Refresh,
            Input::Char('f') => A::Manage(yoctui_model::SetupManageAction::Fragments),
            Input::Char('s') => A::Manage(yoctui_model::SetupManageAction::Status),
            Input::Char('u') => A::Manage(yoctui_model::SetupManageAction::Update),
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

fn management_action(app: &App, key: Input) -> Option<Action> {
    use yoctui_model::SetupManageAction as M;
    let action = if let Some(Dialog::BitBakeSetupOperation(d)) = app.active_dialog() {
        if key == Input::Esc {
            M::Cancel
        } else if d.preparing.is_some() {
            return None;
        } else if d.editor.is_some() {
            match key {
                Input::Char(c) => M::Insert(c.to_string()),
                Input::Backspace => M::Backspace,
                Input::CtrlU => M::Clear,
                Input::Left => M::Motion(TextAreaMotion::Left),
                Input::Right => M::Motion(TextAreaMotion::Right),
                Input::Home => M::Motion(TextAreaMotion::LineStart),
                Input::End => M::Motion(TextAreaMotion::LineEnd),
                Input::Enter => M::AcceptEdit,
                _ => return None,
            }
        } else {
            match key {
                Input::Enter => M::Confirm,
                Input::Char('e') => M::Edit,
                Input::Char('c') => M::Policy,
                Input::Char('b') => M::Backup,
                Input::Char('n') => M::Offline,
                _ => return None,
            }
        }
    } else {
        match key {
            Input::Esc => M::Cancel,
            Input::Char('r') => M::Refresh,
            Input::Up | Input::Char('k') => M::Select(-1),
            Input::Down | Input::Char('j') => M::Select(1),
            Input::PageUp => M::Select(-10),
            Input::PageDown => M::Select(10),
            Input::Home => M::Select(isize::MIN),
            Input::End => M::Select(isize::MAX),
            _ if app.bitbake_setup.management.output.is_some() => return None,
            Input::Enter => M::Show,
            Input::Char('e') => M::Enable,
            Input::Char('d') => M::Disable,
            Input::Char('n') => M::Custom,
            Input::Char('s') => M::Status,
            Input::Char('u') => M::Update,
            _ => return None,
        }
    };
    Some(Action::BitBakeSetup(A::Manage(action)))
}

#[cfg(test)]
#[path = "tests/bitbake_setup.rs"]
mod tests;
