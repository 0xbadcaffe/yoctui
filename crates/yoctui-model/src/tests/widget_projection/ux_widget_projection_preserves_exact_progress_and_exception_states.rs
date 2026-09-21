use super::*;

#[test]
fn ux_widget_projection_preserves_exact_progress_and_exception_states() {
    let progress = GaugeProjection::determinate("Build", 7, 9, WidgetRole::Progress);
    assert_eq!(progress.fraction.unwrap().exact_text(), "7/9 (77%)");
    assert!(progress.text(true, false).contains("7/9 (77%)"));

    let invalid = GaugeProjection::determinate("Build", 10, 9, WidgetRole::Progress);
    assert_eq!(invalid.state, WidgetState::Partial);
    assert!(invalid.text(false, true).contains("10/9 (100%)"));

    let unknown = GaugeProjection::determinate("Build", 0, 0, WidgetRole::Progress);
    assert_eq!(unknown.state, WidgetState::Unknown);
    assert!(unknown.text(false, true).contains("total not reported"));
}
