//! Workbench input is owned by the visible pane, never by hidden global shortcuts.
use crate::{Input, popup_editor_action};
use yoctui_model::{
    Action, App, FocusTarget, PopupEditorCommand, Screen, WorkbenchAction as W,
    WorkbenchFocus as F, WorkbenchFormKind as Form,
};

pub fn workbench_owns_input(app: &App) -> bool {
    app.screen == Screen::Workbench
        && app.focus == FocusTarget::Workspace
        && app.active_dialog().is_none()
        && !app.menu.is_open()
        && !app.command_palette_open
        && !app.onboarding.open
}
pub fn workbench_modal_open(app: &App) -> bool {
    app.workbench.modal_open()
}

pub fn workbench_action(app: &App, key: Input) -> Option<Action> {
    if !workbench_owns_input(app) {
        return None;
    }
    if let Some(dialog) = &app.workbench.debug_dialog {
        let action = match key {
            Input::Esc => W::Cancel,
            _ if dialog.loading => return None,
            Input::Tab | Input::Down => W::DebugField(1),
            Input::BackTab | Input::Up => W::DebugField(-1),
            Input::Enter => W::DebugReview,
            Input::Left | Input::Right | Input::Char(' ') if dialog.field == 0 => W::DebugMode,
            Input::CtrlU => W::DebugClear,
            Input::Backspace => W::DebugBackspace,
            Input::Char(c) => W::DebugText(c),
            _ => return None,
        };
        return Some(Action::Workbench(action));
    }
    if app.workbench.tab_picker.is_some() {
        return Some(Action::Workbench(match key {
            Input::Esc => W::Cancel,
            Input::Enter => W::TabPickerOpen,
            Input::Delete => W::TabPickerClose,
            _ => W::TabPickerSelect(crate::collection_scroll_delta(key)?),
        }));
    }
    let action = if app.workbench.form.is_some() {
        if app.workbench.folder_browser.is_some() {
            return folder_browser_action(app, key).map(Action::Workbench);
        }
        match key {
            Input::Esc => W::Cancel,
            Input::Enter => W::ConfirmForm,
            Input::Backspace => W::FormBackspace,
            Input::CtrlU => W::FormClear,
            Input::Char(c) => W::FormText(c),
            _ => return None,
        }
    } else if app.workbench.menu.is_some() {
        match key {
            Input::Esc => W::Cancel,
            Input::Enter => W::MenuActivate,
            Input::Up | Input::Char('k') => W::MenuSelect(-1),
            Input::Down | Input::Char('j') => W::MenuSelect(1),
            _ => return None,
        }
    } else {
        if app
            .workbench
            .current()
            .is_some_and(|s| s.focus == F::Terminal)
            && !matches!(
                key,
                Input::Alt('m' | 'a' | 'o' | 'f' | 'e' | 't' | 'z' | 'g')
            )
        {
            return None;
        }
        match key {
            Input::Esc => W::Cancel,
            Input::Alt('m') if app.workbench.current().is_some() => W::Menu,
            Input::Alt('a') if app.workbench.current().is_some() => W::Form(Form::AddFolder),
            Input::Alt('o') => W::Sessions,
            Input::Alt('f') => W::Focus(F::Files),
            Input::Alt('e') => W::Focus(F::Editor),
            Input::Alt('t') => W::Terminal,
            Input::Alt('z') => W::TogglePanel,
            Input::Alt('b') => W::RecipeAction(Box::new(Action::BeginSelectedRecipeBuild)),
            Input::Alt('c') => W::Compile,
            Input::Alt('d') => W::RecipeAction(Box::new(Action::BeginSelectedRecipeDevtoolModify)),
            Input::Alt('u') => W::Utility(yoctui_model::DevtoolUtilityCommand::Build),
            Input::Alt('i') => W::Utility(yoctui_model::DevtoolUtilityCommand::IdeSdk),
            Input::Alt('g') => W::Debug,
            Input::Alt('k') => W::ToggleBreakpoint,
            Input::Alt('w') => W::ToggleWhitespace,
            Input::Alt('v') => W::Tabs,
            Input::CtrlG | Input::Alt('n') => W::SearchMatch { backwards: false },
            Input::Alt('p') => W::SearchMatch { backwards: true },
            Input::Alt(c @ '1'..='9') => W::SelectTab(c as usize - '1' as usize),
            Input::F9
                if app
                    .workbench
                    .current()
                    .is_some_and(|s| s.focus == F::Editor) =>
            {
                W::ToggleBreakpoint
            }
            Input::F5
                if app
                    .workbench
                    .current()
                    .is_some_and(|s| s.focus == F::Editor) =>
            {
                W::Debug
            }
            Input::CtrlS => W::Save,
            Input::CtrlF => W::Form(Form::Find),
            _ => return pane_action(app, key).map(Action::Workbench),
        }
    };
    Some(Action::Workbench(action))
}

fn folder_browser_action(app: &App, key: Input) -> Option<W> {
    let browser = app.workbench.folder_browser.as_ref()?;
    Some(match key {
        Input::Esc => W::Cancel,
        Input::Tab | Input::BackTab => W::BrowseFolders,
        Input::Down if browser.editing_path => W::BrowseFolders,
        _ if browser.editing_path => match key {
            Input::Enter => W::ConfirmForm,
            Input::Backspace => W::FormBackspace,
            Input::CtrlU => W::FormClear,
            Input::Char(c) => W::FormText(c),
            _ => return None,
        },
        Input::Enter | Input::Right => W::BrowseOpen,
        Input::Left | Input::Backspace => W::BrowseParent,
        Input::Char(' ') => W::BrowseChoose,
        _ => W::BrowseSelect(crate::collection_scroll_delta(key)?),
    })
}

fn pane_action(app: &App, key: Input) -> Option<W> {
    let Some(session) = app.workbench.current() else {
        return Some(match key {
            Input::Char('n') => W::Form(Form::Create),
            Input::Char('r') => W::Form(Form::Rename),
            Input::Delete => W::RemoveSession,
            Input::Enter | Input::Right => W::Open,
            _ => W::Select(crate::collection_scroll_delta(key)?),
        });
    };
    match session.focus {
        F::Editor => match key {
            Input::Delete => Some(W::Edit(PopupEditorCommand::Delete)),
            Input::Tab => Some(W::Edit(PopupEditorCommand::PasteText {
                text: "\t".into(),
                source: yoctui_model::TextAreaPasteSource::BracketedPaste,
            })),
            _ => match popup_editor_action(true, key)? {
                Action::EditActivePopup(command) => Some(W::Edit(command)),
                _ => None,
            },
        },
        F::Terminal => None,
        F::Files => Some(match key {
            Input::Tab | Input::BackTab => W::CycleFocus,
            Input::Enter | Input::Right => W::Open,
            Input::Left => W::Collapse,
            Input::Char('r') | Input::F5 => W::Refresh,
            Input::Char('.') => W::Hidden,
            Input::Char('n') => W::Form(Form::NewFile),
            Input::Char('[') => W::Folder(-1),
            Input::Char(']') => W::Folder(1),
            _ => W::Select(crate::collection_scroll_delta(key)?),
        }),
        F::Output => matches!(key, Input::Tab | Input::BackTab).then_some(W::CycleFocus),
    }
}
