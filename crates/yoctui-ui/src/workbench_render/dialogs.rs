use super::*;

pub(super) fn render(frame: &mut Frame, app: &App, area: Rect) -> bool {
    let palette = ThemePalette::for_app(app);
    if let Some(d) = &app.workbench.debug_dialog {
        let width = area.width.saturating_sub(2).min(100);
        let height = area.height.min(15);
        let popup = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, popup);
        let mut lines = vec![Line::from(
            "Tab/↑/↓ field · Ctrl+u clear · Enter review · Esc cancel",
        )];
        for (index, (label, value)) in yoctui_model::WORKBENCH_DEBUG_FIELDS
            .iter()
            .zip(d.config.fields())
            .enumerate()
        {
            let available = usize::from(width.saturating_sub(6)).saturating_sub(label.len());
            let value = tail(value, available);
            let text = format!(
                "{} {label}: {value}",
                if index == d.field { "▶" } else { " " }
            );
            lines.push(Line::styled(
                text,
                if index == d.field {
                    selected_style(app, true)
                } else {
                    Style::default()
                },
            ));
        }
        if let Some(error) = &d.error {
            lines.push(Line::styled(
                error.clone(),
                palette.role(palette.error, Modifier::BOLD),
            ));
        }
        if d.loading {
            lines.push(Line::from("Checking ELF and debugger paths… Esc cancels"));
        }
        lines.push(Line::from(
            "Alt+i: prepare SDK. Editor markers apply on GDB launch.",
        ));
        lines.push(Line::from(
            "Local: run. Remote: continue; attaching may pause the target.",
        ));
        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::bordered().title("Workbench GDB · toolchain and launch settings")),
            popup,
        );
        return true;
    }
    if let Some(selection) = app.workbench.tab_picker
        && let Some(s) = app.workbench.current()
    {
        let width = area.width.saturating_sub(4).min(100);
        let height = area.height.min(s.tabs.len() as u16 + 4);
        let popup = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, popup);
        let visible = usize::from(height.saturating_sub(3)).max(1);
        let first = selection.saturating_sub(visible - 1);
        let mut lines = vec![Line::from(
            "↑/↓ choose · Enter activate · Delete close saved tab · Esc cancel",
        )];
        lines.extend(
            s.tabs
                .iter()
                .enumerate()
                .skip(first)
                .take(visible)
                .map(|(index, t)| {
                    Line::styled(
                        format!(
                            "{} {} {} {}",
                            if index == selection { "▶" } else { " " },
                            index + 1,
                            if t.is_dirty() { "●" } else { " " },
                            t.path.display()
                        ),
                        if index == selection {
                            selected_style(app, true)
                        } else {
                            Style::default()
                        },
                    )
                }),
        );
        frame.render_widget(
            Paragraph::new(lines).block(Block::bordered().title("Editor tabs · full paths")),
            popup,
        );
        return true;
    }
    false
}

fn tail(value: &str, width: usize) -> String {
    if unicode_width::UnicodeWidthStr::width(value) <= width {
        return value.into();
    }
    let mut result = Vec::new();
    let mut cells = 1;
    for c in value.chars().rev() {
        let size = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if cells + size > width {
            break;
        }
        cells += size;
        result.push(c);
    }
    format!("…{}", result.into_iter().rev().collect::<String>())
}
