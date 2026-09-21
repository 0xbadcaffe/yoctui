use super::*;

#[test]
fn log_state_reports_bounded_vertical_and_horizontal_positions() {
    let mut logs = LogState::new(10, 1_000);
    logs.insert(log("short"));
    logs.insert(log("a much longer retained line"));
    logs.selection = usize::MAX;
    logs.horizontal_offset = usize::MAX;

    assert_eq!(logs.vertical_position(), Some((2, 2)));
    assert_eq!(logs.horizontal_position(), (26, 26));

    logs.query = "missing".into();
    assert_eq!(logs.vertical_position(), None);
    assert_eq!(logs.horizontal_position(), (0, 0));
}
