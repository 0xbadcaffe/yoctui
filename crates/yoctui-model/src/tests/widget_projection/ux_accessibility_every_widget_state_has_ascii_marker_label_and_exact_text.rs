use super::*;

#[test]
fn ux_accessibility_every_widget_state_has_ascii_marker_label_and_exact_text() {
    for state in [
        WidgetState::Available,
        WidgetState::Active,
        WidgetState::Empty,
        WidgetState::Unknown,
        WidgetState::Unavailable,
        WidgetState::Partial,
        WidgetState::TerminalSuccess,
        WidgetState::TerminalFailure,
        WidgetState::TerminalCancelled,
    ] {
        assert!(state.marker(false, true).is_ascii());
        let projection = GaugeProjection::explicit(
            "Build progress",
            state,
            WidgetRole::Progress,
            "3/10 authoritative tasks",
        );
        let text = projection.text(false, true);
        assert!(text.contains("Build progress"), "{text}");
        if state != WidgetState::Available {
            assert!(text.contains(state.label()), "{text}");
        }
        assert!(text.contains("3/10 authoritative tasks"), "{text}");
    }
    let exact = GaugeProjection::determinate("Build", 3, 10, WidgetRole::Progress);
    assert!(exact.text(false, true).contains("3/10 (30%)"));
}
