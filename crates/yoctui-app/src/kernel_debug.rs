use crate::{Action, App, Input, Screen};
use yoctui_model::{Dialog, FocusTarget, KernelDebugAction as A};

pub fn kernel_debug_owns_input(app: &App, input: Input) -> bool {
    if app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
        || app.platform_menuconfig_visible()
    {
        return false;
    }
    if matches!(app.active_dialog(), Some(Dialog::KernelDebug(_))) {
        return true;
    }
    if matches!(app.active_dialog(), Some(Dialog::TerminalLaunch(dialog)) if Some(&dialog.request) == app.kernel_debug.prepared.as_ref())
    {
        return matches!(input, Input::PageUp | Input::PageDown);
    }
    app.screen == Screen::Kernel
        && app.focus == FocusTarget::Workspace
        && app.active_dialog().is_none()
        && (matches!(input, Input::Char('3') | Input::Char('b'))
            || (app.kernel_debug.visible
                && (action(app, input).is_some() || matches!(input, Input::Char('o' | 'c' | 'd')))))
}

pub fn kernel_debug_action(app: &App, input: Input) -> Option<Action> {
    action(app, input).map(Action::KernelDebug)
}

fn action(app: &App, input: Input) -> Option<A> {
    if matches!(app.active_dialog(), Some(Dialog::TerminalLaunch(dialog)) if Some(&dialog.request) == app.kernel_debug.prepared.as_ref())
    {
        return match input {
            Input::PageUp => Some(A::ScrollPreview(-5)),
            Input::PageDown => Some(A::ScrollPreview(5)),
            _ => None,
        };
    }
    if let Some(Dialog::KernelDebug(dialog)) = app.active_dialog() {
        if input == Input::Esc {
            return Some(A::Cancel);
        }
        if app.kernel_debug.pending.is_some() {
            return None;
        }
        if app.kernel_debug.instrumentation_preview.is_some() {
            return match input {
                Input::Enter => Some(A::Review),
                Input::Up | Input::PageUp => Some(A::ScrollGuide(-3)),
                Input::Down | Input::PageDown => Some(A::ScrollGuide(3)),
                _ => None,
            };
        }
        if dialog.draft.tool.program().is_none() && !dialog.draft.tool.configuration_prep() {
            return match input {
                Input::Up | Input::PageUp => Some(A::ScrollGuide(-3)),
                Input::Down | Input::PageDown => Some(A::ScrollGuide(3)),
                _ => None,
            };
        }
        return match input {
            Input::Enter => Some(A::Review),
            Input::Tab | Input::Down => Some(A::Field(1)),
            Input::BackTab | Input::Up => Some(A::Field(-1)),
            Input::Left | Input::Right => Some(A::ChangeScope),
            Input::Char(' ')
                if dialog
                    .draft
                    .fields()
                    .get(dialog.selection)
                    .is_some_and(|field| {
                        matches!(
                            field,
                            yoctui_model::KernelDebugField::Destination
                                | yoctui_model::KernelDebugField::InstrumentationPreset
                        )
                    }) =>
            {
                Some(A::ChangeScope)
            }
            Input::Char(character) => Some(A::Insert(character.to_string())),
            Input::Backspace => Some(A::Backspace),
            Input::CtrlU => Some(A::Clear),
            Input::PageUp => Some(A::ScrollGuide(-3)),
            Input::PageDown => Some(A::ScrollGuide(3)),
            _ => None,
        };
    }
    match input {
        Input::Char('3') | Input::Char('b') => Some(A::Open),
        Input::Up | Input::Char('k') => Some(A::Select(-1)),
        Input::Down | Input::Char('j') => Some(A::Select(1)),
        Input::PageUp => Some(A::Select(-10)),
        Input::PageDown => Some(A::Select(10)),
        Input::Home => Some(A::Select(isize::MIN)),
        Input::End => Some(A::Select(isize::MAX)),
        Input::Enter | Input::Char('e') => Some(A::OpenSelected),
        Input::Char('r') => Some(A::Inspect),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/kernel_debug.rs"]
mod tests;
