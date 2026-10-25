//! Native performance report; measurements and policy come from typed state.
use super::*;
pub(crate) fn workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let regions = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    let profile = app.build_performance_profile_label();
    frame.render_widget(
        Paragraph::new(format!("Next build: {profile}"))
            .wrap(Wrap { trim: false })
            .block(Block::bordered().title("Build performance · opt-in profile"))
            .style(palette.base()),
        regions[0],
    );
    let rows = app.build_performance_rows();
    let visible = usize::from(regions[1].height.saturating_sub(2)).max(1);
    let offset = app
        .build_performance
        .scroll
        .min(rows.len().saturating_sub(visible));
    let total = rows.len();
    let lines: Vec<Line> = rows
        .into_iter()
        .skip(offset)
        .take(visible)
        .map(|s| {
            let color = if s.starts_with("HOST ")
                || s.starts_with("PRESSURE ")
                || s.starts_with("CACHE ")
                || s.starts_with("MEMORY-")
                || s == "NEXT CONFIRMED BUILD"
            {
                palette.accent
            } else if s.starts_with('>') {
                palette.success
            } else if s.contains("unavailable") || s.starts_with("Partial ") {
                palette.warning
            } else {
                palette.primary_foreground
            };
            Line::styled(s, Style::default().fg(color))
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title(format!(
                "Report · {}–{} / {total} · ↑↓ scroll",
                offset + 1,
                (offset + visible).min(total)
            )))
            .style(palette.base()),
        regions[1],
    );
    frame.render_widget(Paragraph::new("b Balanced · d existing config · 1/2/3 threshold · ←/→ adjust\nPgUp/PgDn Home/End scroll · c Insights · s signatures · Esc navigator")
        .style(palette.base()), regions[2]);
}
