pub(crate) fn shortcut_rail<'a>(app: &App, shortcuts: &'a str) -> Line<'a> {
    let palette = ThemePalette::for_app(app);
    let mut spans = Vec::new();
    for (index, item) in shortcuts.split(" | ").enumerate() {
        if index > 0 {
            spans.push(Span::styled(
                "   ",
                palette.role(palette.disabled, Modifier::DIM),
            ));
        }
        let (key, action) = item.split_once(' ').unwrap_or((item, ""));
        if key == "…" {
            spans.push(Span::styled(
                key,
                palette.role(palette.muted, Modifier::DIM),
            ));
            continue;
        }
        spans.push(Span::styled(
            key,
            palette.role(palette.warning, Modifier::BOLD),
        ));
        if !action.is_empty() {
            spans.push(Span::raw(format!(" {action}")));
        }
    }
    Line::from(spans)
}

pub(crate) fn workbench_footer(frame: &mut Frame, app: &App, area: Rect, _now: SystemTime) {
    let palette = ThemePalette::for_app(app);
    let block = Block::default()
        .borders(if area.height >= 3 {
            Borders::ALL
        } else if area.width == LITERAL_REFERENCE_WIDTH && app.screen == Screen::Tasks {
            Borders::LEFT | Borders::RIGHT | Borders::BOTTOM
        } else {
            Borders::BOTTOM
        })
        .style(palette.base())
        .border_style(Style::default().fg(palette.inactive_border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let shortcuts = if app.preferences.footer_shortcuts {
        footer_rail_shortcuts(app, inner.width)
    } else {
        "F1 Help | shortcuts hidden".into()
    };
    frame.render_widget(
        Paragraph::new(shortcut_rail(app, &shortcuts)).style(palette.base()),
        inner,
    );
}
