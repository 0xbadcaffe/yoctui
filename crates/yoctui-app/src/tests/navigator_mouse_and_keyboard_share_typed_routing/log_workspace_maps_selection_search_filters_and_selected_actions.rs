use super::*;

#[test]
fn log_workspace_maps_selection_search_filters_and_selected_actions() {
    assert_eq!(
        logs_action(false, Input::Up),
        Some(Action::ScrollLogs { delta: 1 })
    );
    assert_eq!(
        logs_action(false, Input::Char('B')),
        Some(Action::CycleLogBuildFilter)
    );
    assert_eq!(
        logs_action(false, Input::Char('o')),
        Some(Action::OpenSelectedLogSource)
    );
    assert_eq!(
        logs_action(false, Input::Char('C')),
        Some(Action::CopySelectedLog)
    );
    assert_eq!(
        logs_action(true, Input::Char('x')),
        Some(Action::AppendLogQuery('x'))
    );
    assert_eq!(logs_action(true, Input::Esc), Some(Action::FinishLogSearch));
}
