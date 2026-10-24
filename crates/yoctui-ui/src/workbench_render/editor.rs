use super::*;

pub(super) fn render(frame: &mut Frame, app: &App, area: Rect) {
    let Some(s) = app.workbench.current() else {
        return;
    };
    let Some(tab) = s.tabs.get(s.tab) else {
        frame.render_widget(Paragraph::new("Open a file from the explorer.\n\nAlt+c Compile · Alt+b Build · Alt+d Modify\nAlt+g GDB · Alt+i SDK/toolchain\nAlt+w Show spaces/tabs · Alt+k Breakpoint\nCtrl+f Find · Ctrl+g Next · Alt+p Previous\nAlt+v Tab list · Alt+1..9 Select tab\nCtrl+w Close tab · Ctrl+s Save")
            .block(Block::bordered().title("Editor")).wrap(Wrap { trim: false }), area);
        return;
    };
    let palette = ThemePalette::for_app(app);
    let p = tab.document.position();
    let language = SourceLanguage::from_source(&tab.path, &tab.document.text);
    let search = tab.document.search_state();
    let find = if search.query.is_empty() {
        String::new()
    } else {
        format!(
            " · Find {}/{}{}",
            search.selected.map_or(0, |n| n + 1),
            search.matches.len(),
            if search.truncated { "+" } else { "" }
        )
    };
    let block = Block::bordered()
        .title(format!(
            "{} · {language:?} · Ln {} Col {} · {}{find}{}",
            tab.path
                .strip_prefix(&tab.root)
                .unwrap_or(&tab.path)
                .display(),
            p.line + 1,
            p.column + 1,
            if tab.is_dirty() { "unsaved" } else { "saved" },
            if s.show_whitespace {
                " · whitespace"
            } else {
                ""
            }
        ))
        .border_style(if s.focus == Focus::Editor {
            palette.role(palette.focused_border, Modifier::BOLD)
        } else {
            Style::default()
        });
    let footer_find = if find.is_empty() {
        String::new()
    } else {
        format!("{} · ", find.trim_start_matches(" · "))
    };
    let block = block.title_bottom(format!(
        "{footer_find}BP {} staged · Alt+w whitespace",
        tab.breakpoints.len()
    ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let gutter = yoctui_model::workbench_gutter_width(tab.document.line_count())
        .min(usize::from(inner.width));
    let cached = crate::source_render::cached_source_preview(
        &tab.document.text,
        &tab.path.to_string_lossy(),
        app,
    );
    let mut offset = tab
        .document
        .text
        .split_inclusive('\n')
        .take(tab.viewport.top)
        .map(str::len)
        .sum::<usize>();
    let mut gutters = Vec::new();
    let mut lines = Vec::new();
    for (index, raw) in tab
        .document
        .text
        .split('\n')
        .enumerate()
        .skip(tab.viewport.top)
        .take(usize::from(inner.height))
    {
        let marker = if tab.breakpoints.contains(&(index + 1)) {
            "●"
        } else {
            " "
        };
        gutters.push(Line::styled(
            format!(
                "{marker}{:>width$} │ ",
                index + 1,
                width = gutter.saturating_sub(4)
            ),
            palette.role(
                if marker == "●" {
                    palette.error
                } else {
                    palette.secondary_foreground
                },
                Modifier::empty(),
            ),
        ));
        let fallback = Line::from(raw.to_owned());
        let syntax = cached.lines.get(index).unwrap_or(&fallback);
        let mut spans = Vec::new();
        let mut column = 0;
        let mut byte = offset;
        for span in &syntax.spans {
            for c in span.content.chars() {
                if byte == tab.document.cursor {
                    spans.push(Span::styled(
                        "▏",
                        palette.role(palette.informational, Modifier::BOLD),
                    ));
                }
                let selected = tab
                    .document
                    .selection
                    .is_some_and(|(start, end)| start <= byte && byte < end);
                let match_index = search.matches.partition_point(|(start, _)| *start <= byte);
                let found = match_index
                    .checked_sub(1)
                    .is_some_and(|n| byte < search.matches[n].1);
                let mut style = span.style;
                if selected {
                    style = style.patch(selected_style(app, true));
                } else if found {
                    style = style
                        .bg(palette.warning)
                        .fg(palette.background)
                        .add_modifier(Modifier::UNDERLINED);
                }
                let count = if c == '\t' {
                    yoctui_model::WORKBENCH_TAB_WIDTH - column % yoctui_model::WORKBENCH_TAB_WIDTH
                } else {
                    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
                };
                if c == '\t' || c == ' ' {
                    let value = if c == '\t' {
                        format!(
                            "{}{}",
                            if s.show_whitespace { "→" } else { " " },
                            " ".repeat(count.saturating_sub(1))
                        )
                    } else if s.show_whitespace {
                        "·".into()
                    } else {
                        " ".into()
                    };
                    if s.show_whitespace && !selected && !found {
                        style = style.fg(palette.syntax_comment);
                    }
                    spans.push(Span::styled(value, style));
                } else {
                    spans.push(Span::styled(c.to_string(), style));
                }
                column += count;
                byte += c.len_utf8();
            }
        }
        if raw.ends_with('\r') && byte < offset + raw.len() {
            if byte == tab.document.cursor {
                spans.push(Span::styled(
                    "▏",
                    palette.role(palette.informational, Modifier::BOLD),
                ));
            }
            if s.show_whitespace {
                spans.push(Span::styled(
                    "␍",
                    palette.role(palette.syntax_comment, Modifier::empty()),
                ));
            }
        }
        if offset + raw.len() == tab.document.cursor {
            spans.push(Span::styled(
                "▏",
                palette.role(palette.informational, Modifier::BOLD),
            ));
        }
        offset += raw.len() + 1;
        lines.push(Line::from(spans));
    }
    frame.render_widget(
        Paragraph::new(gutters),
        Rect::new(inner.x, inner.y, gutter as u16, inner.height),
    );
    frame.render_widget(
        Paragraph::new(lines).scroll((0, tab.viewport.left.min(u16::MAX as usize) as u16)),
        Rect::new(
            inner.x + gutter as u16,
            inner.y,
            inner.width.saturating_sub(gutter as u16),
            inner.height,
        ),
    );
    let mut scrollbar = ratatui::widgets::ScrollbarState::new(tab.document.line_count())
        .position(tab.viewport.top)
        .viewport_content_length(usize::from(inner.height));
    frame.render_stateful_widget(
        ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight),
        area,
        &mut scrollbar,
    );
}
