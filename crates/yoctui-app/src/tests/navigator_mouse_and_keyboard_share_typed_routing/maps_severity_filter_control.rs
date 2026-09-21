use super::*;

#[test]
fn maps_severity_filter_control() {
    assert_eq!(key_action(Input::Char('s')), Some(Action::CycleLogSeverity));
}
