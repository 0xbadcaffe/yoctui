//! Application-owned input mapping, keeping terminal concerns outside the reducer.
use yoctui_model::{Action, Screen};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Esc,
    Enter,
    CtrlC,
    CtrlB,
    Tab,
    BackTab,
    CtrlS,
    Up,
    Down,
    Backspace,
    Left,
    Right,
}
pub fn key_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('b') => None,
        Input::Char('c') => Some(Action::Cancel),
        Input::Char('f') => Some(Action::ToggleLogFollow),
        Input::Char('w') => Some(Action::ToggleLogWrap),
        Input::Char('s') => Some(Action::CycleLogSeverity),
        Input::Char('/') => Some(Action::BeginLogSearch),
        Input::Char('n') => Some(Action::NextLogMatch),
        Input::Char('N') => Some(Action::PreviousLogMatch),
        Input::Char('R') => Some(Action::CycleLogRecipeFilter),
        Input::Char('T') => Some(Action::CycleLogTaskFilter),
        Input::Backspace => Some(Action::BackspaceLogQuery),
        Input::Up => Some(Action::ScrollLogs { delta: 1 }),
        Input::Down => Some(Action::ScrollLogs { delta: -1 }),
        Input::Left => Some(Action::ScrollLogsHorizontally { delta: -8 }),
        Input::Right => Some(Action::ScrollLogsHorizontally { delta: 8 }),
        Input::Char('l') => Some(Action::Open(Screen::Logs)),
        Input::Char('h') => Some(Action::Open(Screen::BuildHistory)),
        Input::Char('e') => Some(Action::Open(Screen::Errors)),
        Input::Char('r') => Some(Action::Open(Screen::Recipes)),
        Input::Char('y') => Some(Action::Open(Screen::Layers)),
        Input::Char('v') => Some(Action::Open(Screen::Configuration)),
        Input::Char('x') => Some(Action::Open(Screen::Bbmask)),
        Input::Char('?') => Some(Action::Open(Screen::Help)),
        Input::Char('q') | Input::CtrlC => Some(Action::Quit),
        Input::Tab => Some(Action::CycleFocus { backwards: false }),
        Input::BackTab => Some(Action::CycleFocus { backwards: true }),
        Input::Char('Y') => Some(Action::ConfirmQuit),
        Input::Enter => Some(Action::DismissNotification),
        Input::Esc => Some(Action::Open(Screen::Dashboard)),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_navigation() {
        assert_eq!(
            key_action(Input::Char('l')),
            Some(Action::Open(Screen::Logs))
        );
        assert_eq!(
            key_action(Input::Char('x')),
            Some(Action::Open(Screen::Bbmask))
        );
    }
    #[test]
    fn maps_log_controls() {
        assert_eq!(key_action(Input::Char('f')), Some(Action::ToggleLogFollow));
        assert_eq!(key_action(Input::Char('w')), Some(Action::ToggleLogWrap));
        assert_eq!(key_action(Input::Up), Some(Action::ScrollLogs { delta: 1 }));
    }
    #[test]
    fn enter_dismisses_notification() {
        assert_eq!(key_action(Input::Enter), Some(Action::DismissNotification));
    }
    #[test]
    fn maps_severity_filter_control() {
        assert_eq!(key_action(Input::Char('s')), Some(Action::CycleLogSeverity));
    }
    #[test]
    fn maps_recipe_and_task_filter_controls() {
        assert_eq!(
            key_action(Input::Char('R')),
            Some(Action::CycleLogRecipeFilter)
        );
        assert_eq!(
            key_action(Input::Char('T')),
            Some(Action::CycleLogTaskFilter)
        );
    }
    #[test]
    fn maps_log_match_navigation_controls() {
        assert_eq!(key_action(Input::Char('n')), Some(Action::NextLogMatch));
        assert_eq!(key_action(Input::Char('N')), Some(Action::PreviousLogMatch));
    }
}
