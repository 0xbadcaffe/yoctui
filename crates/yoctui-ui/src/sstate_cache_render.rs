//! Cache report projection; no filesystem traversal or network checks during rendering.
use super::*;
pub(crate) fn workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let regions = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(app.sstate_cache_build_summary())
            .wrap(Wrap { trim: false })
            .block(Block::bordered().title("Maintenance · Sstate cache performance")),
        regions[0],
    );
    let rows = app.sstate_cache_rows();
    let visible = usize::from(regions[1].height.saturating_sub(2)).max(1);
    let offset = app
        .sstate_cache
        .scroll
        .min(rows.len().saturating_sub(visible));
    let total = rows.len();
    let lines: Vec<Line> = rows
        .into_iter()
        .skip(offset)
        .take(visible)
        .map(|s| {
            let color = if s.contains("unavailable") {
                palette.warning
            } else if s.starts_with("SSTATE ")
                || s.starts_with("CURRENT ")
                || s.starts_with("NEXT ")
                || s.starts_with("EXISTING ")
            {
                palette.accent
            } else {
                palette.primary_foreground
            };
            Line::styled(s, Style::default().fg(color))
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(format!(
            "Cache report · {}–{} / {total}",
            offset + 1,
            (offset + visible).min(total)
        ))),
        regions[1],
    );
    frame.render_widget(Paragraph::new("e Configure · u Existing config · r Inspect · c Readiness · d Cleanup\nt Tools · [/] Tabs · ↑/↓ PgUp/PgDn Home/End Scroll").style(palette.base()),regions[2]);
}
pub(crate) fn review(
    frame: &mut Frame,
    app: &App,
    profile: &yoctui_model::SstateCacheProfile,
    area: Rect,
) {
    let popup = bounded_dialog_rect(area, 100, 20);
    let block = Block::bordered().title("Review sstate cache profile");
    let inner = block.inner(popup);
    let regions = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(inner);
    let rows = profile.review_rows(usize::from(regions[0].width));
    let visible = usize::from(regions[0].height).max(1);
    let offset = app
        .sstate_cache
        .review_scroll
        .min(rows.len().saturating_sub(visible));
    let lines: Vec<Line> = rows
        .into_iter()
        .skip(offset)
        .take(visible)
        .map(Line::raw)
        .collect();
    frame.render_widget(Clear, popup);
    frame.render_widget(block.style(ThemePalette::for_app(app).base()), popup);
    frame.render_widget(Paragraph::new(lines), regions[0]);
    frame.render_widget(
        Paragraph::new("Enter Save profile · Esc Cancel\n↑/↓ PgUp/PgDn Home/End Scroll"),
        regions[1],
    );
}
