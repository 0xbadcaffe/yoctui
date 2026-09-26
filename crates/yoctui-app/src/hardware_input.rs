//! Keyboard controls for the persistent Hardware library and viewer.

use crate::Input;
use std::path::PathBuf;
use yoctui_model::{Action, App, HardwareAction};

pub fn hardware_workspace_action(app: &App, input: Input) -> Option<Action> {
    let state = &app.hardware;
    if state.removal_pending.is_some() {
        return match input {
            Input::Enter | Input::Char('y') => hardware(HardwareAction::ConfirmRemove),
            Input::Esc | Input::Char('n') => hardware(HardwareAction::CancelRemove),
            _ => None,
        };
    }
    if let Some(viewer) = state.viewer.as_ref() {
        if viewer.searching {
            return match input {
                Input::Esc | Input::Enter => hardware(HardwareAction::FinishSearch),
                Input::Backspace => hardware(HardwareAction::BackspaceSearch),
                Input::Char(character) => hardware(HardwareAction::AppendSearch(character)),
                _ => None,
            };
        }
        return match input {
            Input::Esc => hardware(HardwareAction::CloseViewer),
            Input::PageUp | Input::Char('[') => hardware(HardwareAction::ChangePage { delta: -1 }),
            Input::PageDown | Input::Char(']') => hardware(HardwareAction::ChangePage { delta: 1 }),
            Input::Home => hardware(HardwareAction::FirstPage),
            Input::End => hardware(HardwareAction::LastPage),
            Input::Char('+') => hardware(HardwareAction::Zoom { delta: 25 }),
            Input::Char('-') => hardware(HardwareAction::Zoom { delta: -25 }),
            Input::Char('0') => hardware(HardwareAction::ResetZoom),
            Input::Left | Input::Char('h') => hardware(HardwareAction::Pan {
                horizontal: -4,
                vertical: 0,
            }),
            Input::Right | Input::Char('l') => hardware(HardwareAction::Pan {
                horizontal: 4,
                vertical: 0,
            }),
            Input::Up | Input::Char('k') => hardware(HardwareAction::Pan {
                horizontal: 0,
                vertical: -2,
            }),
            Input::Down | Input::Char('j') => hardware(HardwareAction::Pan {
                horizontal: 0,
                vertical: 2,
            }),
            Input::Char('/') => hardware(HardwareAction::BeginSearch),
            Input::Char('n') => hardware(HardwareAction::NextMatch { backwards: false }),
            Input::Char('N') => hardware(HardwareAction::NextMatch { backwards: true }),
            Input::Char('r') => hardware(HardwareAction::Reload),
            _ => None,
        };
    }
    if let Some(browser) = state.browser.as_ref() {
        return match input {
            Input::Esc => hardware(HardwareAction::CancelBrowser),
            Input::Up | Input::Char('k') => {
                hardware(HardwareAction::SelectBrowserEntry { delta: -1 })
            }
            Input::Down | Input::Char('j') => {
                hardware(HardwareAction::SelectBrowserEntry { delta: 1 })
            }
            Input::PageUp => hardware(HardwareAction::SelectBrowserEntry { delta: -10 }),
            Input::PageDown => hardware(HardwareAction::SelectBrowserEntry { delta: 10 }),
            Input::Left => hardware(HardwareAction::SelectBrowserCategory { delta: -1 }),
            Input::Right => hardware(HardwareAction::SelectBrowserCategory { delta: 1 }),
            Input::Backspace => hardware(HardwareAction::BrowseParent),
            Input::Enter => browser.entries.get(browser.selection).map_or_else(
                || hardware(HardwareAction::EnterBrowserEntry),
                |entry| {
                    hardware(if entry.is_directory {
                        HardwareAction::EnterBrowserEntry
                    } else {
                        HardwareAction::ConfirmAdd
                    })
                },
            ),
            Input::Char('a') if !browser.loading => hardware(HardwareAction::ConfirmAdd),
            _ => None,
        };
    }
    match input {
        Input::Up | Input::Char('k') => hardware(HardwareAction::SelectDocument { delta: -1 }),
        Input::Down | Input::Char('j') => hardware(HardwareAction::SelectDocument { delta: 1 }),
        Input::PageUp => hardware(HardwareAction::SelectDocument { delta: -10 }),
        Input::PageDown => hardware(HardwareAction::SelectDocument { delta: 10 }),
        Input::Home => hardware(HardwareAction::SelectDocument { delta: isize::MIN }),
        Input::End => hardware(HardwareAction::SelectDocument { delta: isize::MAX }),
        Input::Left => hardware(HardwareAction::SelectCategory { delta: -1 }),
        Input::Right | Input::Tab => hardware(HardwareAction::SelectCategory { delta: 1 }),
        Input::BackTab => hardware(HardwareAction::SelectCategory { delta: -1 }),
        Input::Enter => hardware(HardwareAction::OpenSelected),
        Input::Char('r') => hardware(HardwareAction::OpenSelected),
        Input::Char('a') => hardware(HardwareAction::OpenBrowser {
            directory: state
                .last_directory
                .clone()
                .unwrap_or_else(|| PathBuf::from("/")),
        }),
        Input::Char('d') => hardware(HardwareAction::BeginRemove),
        _ => None,
    }
}

fn hardware(action: HardwareAction) -> Option<Action> {
    Some(Action::Hardware(action))
}
