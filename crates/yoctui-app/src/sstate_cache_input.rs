use crate::{Input, popup_editor_action};
use yoctui_model::{
    Action, App, Dialog, FocusTarget, MaintenanceView, Screen, SstateCacheAction as C,
    SstateCacheDialog,
};
pub fn sstate_cache_action(app: &App, key: Input) -> Option<Action> {
    if let Some(Dialog::SstateCache(dialog)) = app.active_dialog() {
        return match dialog.as_ref() {
            SstateCacheDialog::Configure { editor, .. } => match key {
                Input::Enter if !editor.editing => Some(Action::SstateCache(C::Review)),
                Input::Esc | Input::Char('q') if !editor.editing => {
                    Some(Action::SstateCache(C::Cancel))
                }
                key => popup_editor_action(editor.editing, key),
            },
            SstateCacheDialog::Review(_) => match key {
                Input::Enter => Some(Action::SstateCache(C::Save)),
                Input::Up | Input::Char('k') => Some(Action::SstateCache(C::Scroll(-1))),
                Input::Down | Input::Char('j') => Some(Action::SstateCache(C::Scroll(1))),
                Input::PageUp => Some(Action::SstateCache(C::Scroll(
                    -(app.sstate_cache.review_visible_rows.max(1) as isize),
                ))),
                Input::PageDown => Some(Action::SstateCache(C::Scroll(
                    app.sstate_cache.review_visible_rows.max(1) as isize,
                ))),
                Input::Home => Some(Action::SstateCache(C::First)),
                Input::End => Some(Action::SstateCache(C::Last)),
                Input::Esc | Input::Char('q') => Some(Action::SstateCache(C::Cancel)),
                _ => None,
            },
        };
    }
    if app.screen != Screen::Maintenance
        || app.maintenance.view != MaintenanceView::Sstate
        || !app.sstate_cache.dashboard
        || app.focus != FocusTarget::Workspace
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
    {
        return None;
    }
    Some(match key {
        Input::Char('e') => Action::SstateCache(C::Configure),
        Input::Char('u') => Action::SstateCache(C::Disable),
        Input::Char('t') | Input::Esc => Action::SstateCache(C::Tools),
        Input::Up | Input::Char('k') => Action::SstateCache(C::Scroll(-1)),
        Input::Down | Input::Char('j') => Action::SstateCache(C::Scroll(1)),
        Input::PageUp => {
            Action::SstateCache(C::Scroll(-(app.sstate_cache.visible_rows.max(1) as isize)))
        }
        Input::PageDown => {
            Action::SstateCache(C::Scroll(app.sstate_cache.visible_rows.max(1) as isize))
        }
        Input::Home => Action::SstateCache(C::First),
        Input::End => Action::SstateCache(C::Last),
        _ => return None,
    })
}
#[cfg(test)]
#[path = "tests/sstate_cache_input.rs"]
mod tests;
