use super::*;

#[test]
fn cycles_log_severity_filter() {
    let mut app = App::new(2, 10);
    for expected in [
        Some(Severity::Info),
        Some(Severity::Warning),
        Some(Severity::Error),
        None,
    ] {
        let _ = update(&mut app, Action::CycleLogSeverity);
        assert_eq!(app.logs.filter, expected);
    }
}
