//! Performance shortcuts never escape their visible, focused workspace.
use crate::Input;
use yoctui_model::{Action, App, BuildPerformanceAction as P, FocusTarget, Screen};
pub fn build_performance_action(app: &App, input: Input) -> Option<Action> {
    if app.screen != Screen::BuildPerformance
        || app.focus != FocusTarget::Workspace
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
        || app.keymap_preferences_ui.open
    {
        return None;
    }
    let action = match input {
        Input::Char('b') => P::Balanced,
        Input::Char('d') => P::Defaults,
        Input::Char(n @ '1'..='3') => {
            P::Field(i32::from(n as u8 - b'1') - app.build_performance.field as i32)
        }
        Input::Left => P::Adjust(-5000),
        Input::Right => P::Adjust(5000),
        Input::Up | Input::Char('k') => P::Scroll(-1),
        Input::Down | Input::Char('j') => P::Scroll(1),
        Input::PageUp => P::Scroll(-(app.build_performance.visible_rows.max(1) as i32)),
        Input::PageDown => P::Scroll(app.build_performance.visible_rows.max(1) as i32),
        Input::Home => P::First,
        Input::End => P::Last,
        Input::Char('c') => {
            return Some(Action::SstateCache(yoctui_model::SstateCacheAction::Open));
        }
        Input::Char('s') => return Some(Action::Open(Screen::Signatures)),
        Input::Esc => return Some(Action::Focus(FocusTarget::Navigator)),
        _ => return None,
    };
    Some(Action::BuildPerformance(action))
}
#[cfg(test)]
#[path = "tests/build_performance_input.rs"]
mod tests;
