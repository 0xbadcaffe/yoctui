//! Menu render.
use super::*;

pub(crate) fn menu_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let items = app.active_menu_items();
    if app.menu.kind == Some(yoctui_model::MenuKind::Application)
        && area.width >= 150
        && area.height >= 50
    {
        application_menu_overlay(frame, app, area, &items);
        return;
    }
    let width = area.width.saturating_sub(4).min(94);
    let height = area
        .height
        .saturating_sub(2)
        .min(
            u16::try_from(items.len())
                .unwrap_or(u16::MAX)
                .saturating_add(10),
        )
        .min(28);
    if width < 36 || height < 10 {
        return;
    }
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    let palette = ThemePalette::for_app(app);
    let title = match app.menu.kind {
        Some(yoctui_model::MenuKind::Application) => "Application menu · focus trapped".into(),
        Some(yoctui_model::MenuKind::Context(destination)) => {
            format!("{} actions · focus trapped", destination.label())
        }
        None => return,
    };
    let outer = dialog_block(app, title, DialogTone::Standard);
    let inner = outer.inner(popup);
    frame.render_widget(outer, popup);
    if inner.is_empty() {
        return;
    }
    let regions = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(4),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .split(inner);
    let groups = yoctui_model::ApplicationMenuGroup::ALL
        .iter()
        .enumerate()
        .map(|(index, group)| {
            if app.menu.kind == Some(yoctui_model::MenuKind::Application)
                && index == app.menu.group_selection
            {
                format!("[{}]", group.label())
            } else {
                group.label().to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("  ");
    let prefix = if app.menu.typed_prefix.is_empty() {
        "type-ahead: <empty>".into()
    } else {
        format!("type-ahead: {}_", app.menu.typed_prefix)
    };
    frame.render_widget(
        Paragraph::new(vec![Line::raw(groups), Line::raw(prefix)]).style(palette.base()),
        regions[0],
    );

    let selected = app.menu.item_selection.min(items.len().saturating_sub(1));
    let visible = usize::from(regions[1].height.saturating_sub(3)).max(1);
    let start = selected
        .saturating_sub(visible / 2)
        .min(items.len().saturating_sub(visible));
    let catalog_title = BoundedScrollIndicator::new(start, visible, items.len())
        .title_label(
            (!items.is_empty()).then_some(selected),
            true,
            app.preferences.symbols == SymbolPreference::Unicode,
        )
        .map_or_else(
            || "Catalog actions".into(),
            |cue| format!("Catalog actions · {cue}"),
        );
    let rows = items
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(index, item)| {
            let availability = item.disabled_reason.as_deref().unwrap_or("available");
            let safety = match item.safety {
                yoctui_model::OperatorActionSafety::ReadOnly => "read-only",
                yoctui_model::OperatorActionSafety::ConfirmationRequired => "confirm",
                yoctui_model::OperatorActionSafety::DestructiveConfirmation => "DESTRUCTIVE",
            };
            Row::new([
                format!(
                    "{} {}",
                    if index == selected { ">" } else { " " },
                    item.label
                ),
                item.shortcut.to_owned(),
                safety.to_owned(),
                availability.to_owned(),
            ])
            .style(if index == selected {
                selected_style(app, true)
            } else if item.enabled() {
                palette.base()
            } else {
                palette.role(palette.disabled, Modifier::DIM)
            })
        });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(34),
                Constraint::Length(14),
                Constraint::Length(12),
                Constraint::Percentage(45),
            ],
        )
        .header(
            Row::new(["Action", "Shortcut", "Safety", "Availability"])
                .style(palette.role(palette.heading, Modifier::BOLD)),
        )
        .block(Block::default().title(catalog_title).borders(Borders::ALL)),
        regions[1],
    );

    let detail = items.get(selected).map_or_else(
        || "No contextual catalog actions are available for this workspace.".into(),
        |item| {
            format!(
                "{}\n{} · {}",
                item.description,
                item.action_id.as_str(),
                item.disabled_reason
                    .as_deref()
                    .unwrap_or("available through the existing typed route")
            )
        },
    );
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .title("Selected action")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        regions[2],
    );
    frame.render_widget(
        Paragraph::new(bounded_cell_text(
            "Esc/F10 close · Enter activate · ↑/↓ items · ←/→ groups · type prefix · Backspace",
            regions[3].width,
        ))
        .style(palette.role(palette.secondary_foreground, Modifier::DIM)),
        regions[3],
    );
}

pub(crate) fn application_menu_overlay(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    items: &[yoctui_model::MenuItem],
) {
    let palette = ThemePalette::for_app(app);
    let width = 60.min(area.width.saturating_sub(4));
    let height = u16::try_from(items.len())
        .unwrap_or(u16::MAX)
        .saturating_add(5)
        .clamp(10, 18)
        .min(area.height.saturating_sub(8));
    let header = yoctui_app::workbench_chrome_heights(app, area.width, area.height)[0];
    let anchor = (area.width / 4).min(area.width.saturating_sub(width));
    let popup = Rect::new(anchor, header, width, height);
    clear_popup(frame, app, popup);
    let selected = app.menu.item_selection.min(items.len().saturating_sub(1));
    let item_viewport_height = usize::from(height.saturating_sub(5)).max(1);
    let viewport = yoctui_model::centered_viewport_range(
        (!items.is_empty()).then_some(selected),
        items.len(),
        item_viewport_height,
    );
    let title = BoundedScrollIndicator::new(viewport.start, viewport.len(), items.len())
        .title_label(
            (!items.is_empty()).then_some(selected),
            true,
            app.preferences.symbols == SymbolPreference::Unicode,
        )
        .map_or_else(
            || "Application menu · focus trapped".into(),
            |cue| format!("Application menu · {cue} · focus trapped"),
        );
    let outer = dialog_block(app, title, DialogTone::Standard);
    let inner = outer.inner(popup);
    frame.render_widget(outer, popup);
    if inner.is_empty() {
        return;
    }
    let regions = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(2),
        Constraint::Length(1),
    ])
    .split(inner);
    let groups = yoctui_model::ApplicationMenuGroup::ALL
        .iter()
        .enumerate()
        .map(|(index, group)| {
            if index == app.menu.group_selection {
                Span::styled(format!(" {} ", group.label()), palette.selected())
            } else {
                Span::raw(format!(" {} ", group.label()))
            }
        })
        .collect::<Vec<_>>();
    let prefix = if app.menu.typed_prefix.is_empty() {
        "Type to jump".into()
    } else {
        format!("Jump: {}_", app.menu.typed_prefix)
    };
    frame.render_widget(
        Paragraph::new(vec![Line::from(groups), Line::from(prefix)]),
        regions[0],
    );
    debug_assert_eq!(item_viewport_height, usize::from(regions[1].height).max(1));
    let rows = items[viewport.clone()]
        .iter()
        .enumerate()
        .map(|(offset, item)| {
            let index = viewport.start + offset;
            let suffix = item
                .disabled_reason
                .as_deref()
                .unwrap_or(item.description.as_str());
            Row::new([
                format!(
                    "{} {}",
                    if index == selected { ">" } else { " " },
                    item.label
                ),
                item.shortcut.to_owned(),
                suffix.to_owned(),
            ])
            .style(if index == selected {
                selected_style(app, true)
            } else if item.enabled() {
                palette.base()
            } else {
                palette.role(palette.disabled, Modifier::DIM)
            })
        });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(44),
                Constraint::Length(10),
                Constraint::Percentage(40),
            ],
        ),
        regions[1],
    );
    frame.render_widget(
        Paragraph::new("↑/↓ Select · Enter Activate · Esc Close · ←/→ Menu")
            .style(palette.role(palette.secondary_foreground, Modifier::DIM)),
        regions[2],
    );
}
