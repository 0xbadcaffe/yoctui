use super::*;

#[test]
fn ux_logs_keyboard_routes_source_time_bookmarks_and_bounded_export() {
    for (input, expected) in [
        (Input::Char('S'), Action::CycleLogSourceFilter),
        (Input::Char('I'), Action::CycleLogTimeRange),
        (Input::Char('m'), Action::ToggleSelectedLogBookmark),
        (Input::Char(']'), Action::NextLogBookmark),
        (Input::Char('['), Action::PreviousLogBookmark),
        (Input::Char('E'), Action::ExportFilteredLogs),
    ] {
        assert_eq!(logs_action(false, input), Some(expected));
    }
    assert_eq!(
        logs_action(true, Input::Char('m')),
        Some(Action::AppendLogQuery('m'))
    );
}
