pub(crate) fn collect_terminal_panes(
    node: &PaneNode,
    area: Rect,
    output: &mut Vec<(Rect, yoctui_model::PaneId)>,
) {
    match node {
        PaneNode::Leaf { id } => output.push((area, *id)),
        PaneNode::Split {
            axis,
            ratio_per_mille,
            first,
            second,
        } => {
            let ratio = f32::from(*ratio_per_mille) / 1000.0;
            let (first_area, second_area) = match axis {
                SplitAxis::Horizontal => {
                    let first_width = (f32::from(area.width) * ratio) as u16;
                    let first_width = first_width.max(1).min(area.width.saturating_sub(1));
                    (
                        Rect {
                            width: first_width,
                            ..area
                        },
                        Rect {
                            x: area.x + first_width,
                            width: area.width - first_width,
                            ..area
                        },
                    )
                }
                SplitAxis::Vertical => {
                    let first_height = (f32::from(area.height) * ratio) as u16;
                    let first_height = first_height.max(1).min(area.height.saturating_sub(1));
                    (
                        Rect {
                            height: first_height,
                            ..area
                        },
                        Rect {
                            y: area.y + first_height,
                            height: area.height - first_height,
                            ..area
                        },
                    )
                }
            };
            collect_terminal_panes(first, first_area, output);
            collect_terminal_panes(second, second_area, output);
        }
    }
}

pub(crate) fn pane_switcher(frame: &mut Frame, app: &App, area: Rect) {
    let label = |target: FocusTarget, name: &str| {
        if app.focus == target {
            format!("[{name}]")
        } else {
            name.to_owned()
        }
    };
    let panes = yoctui_model::pane_focus_targets(app)
        .map(|target| label(target, target.label()))
        .collect::<Vec<_>>()
        .join("  ");
    frame.render_widget(
        Paragraph::new(format!("Panes: {panes}  Tab/Shift+Tab"))
            .style(ThemePalette::for_app(app).focus()),
        area,
    );
}

pub(crate) fn pane_block<'a>(app: &App, title: &'a str, focused: bool) -> Block<'a> {
    let palette = ThemePalette::for_app(app);
    PaneShell::new(
        Line::styled(title, palette.role(palette.informational, Modifier::BOLD)),
        focused,
        pane_styles(app),
    )
    .block()
}

pub(crate) fn pane_styles(app: &App) -> PaneStyles {
    let palette = ThemePalette::for_app(app);
    PaneStyles {
        base: palette.base(),
        border: Style::default().fg(palette.inactive_border),
        focused_border: palette.focus(),
        selected: palette.selected(),
        inactive_selected: if palette.attribute_only {
            Style::default().add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(palette.selection_foreground)
        },
        table_header: palette.role(palette.table_header, Modifier::BOLD),
        muted: palette.role(palette.muted, Modifier::DIM),
    }
}
