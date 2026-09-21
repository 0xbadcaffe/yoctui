use super::*;

#[test]
fn ux_internal_log_keyboard_routes_only_the_separate_diagnostic_authority() {
    for (input, expected) in [
        (Input::Char('v'), Action::CycleLogWorkspaceView),
        (Input::Char('f'), Action::ToggleInternalLogFollow),
        (Input::Char('s'), Action::CycleInternalLogLevelFilter),
        (Input::Char('T'), Action::CycleInternalLogTargetFilter),
        (Input::Char('/'), Action::BeginInternalLogSearch),
        (Input::Char('c'), Action::ClearInternalLogs),
        (Input::Char('E'), Action::ExportInternalLogs),
    ] {
        assert_eq!(internal_logs_action(false, input), Some(expected));
    }
    assert_eq!(
        internal_logs_action(true, Input::Char('x')),
        Some(Action::AppendInternalLogQuery('x'))
    );
    assert_eq!(internal_logs_action(false, Input::Char('B')), None);
}
