use super::*;

#[test]
fn ux_widget_projection_bounds_large_histories_bars_tabs_and_legends() {
    let history = HistoryProjection::bounded(
        "CPU",
        WidgetState::Available,
        WidgetRole::Cpu,
        Some(999),
        0..10_000,
        60,
        "60-sample history",
    );
    assert_eq!(history.points.len(), 60);
    assert_eq!(history.points[0], 9_940);

    let bars = BarProjection::bounded(
        WidgetState::Available,
        (0..1_000).map(|value| BarValue {
            label: format!("包{value}"),
            value,
            role: WidgetRole::Accent,
        }),
        8,
        "",
    );
    assert_eq!(bars.values.len(), 8);

    let tabs = TabProjection::bounded(
        (0..1_000).map(|index| format!("Tab {index}")),
        usize::MAX,
        6,
    );
    assert_eq!(tabs.selected, Some(5));

    let legend = LegendProjection::bounded(
        WidgetState::Available,
        bars.values.iter().map(|bar| LegendItem {
            label: bar.label.clone(),
            value: bar.value.to_string(),
            role: bar.role,
        }),
        3,
        "",
    );
    assert_eq!(legend.items.len(), 3);
}
