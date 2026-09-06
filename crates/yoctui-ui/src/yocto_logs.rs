//! Upstream log-widget presentation, never a domain log/diagnostic authority.
//!
//! Only the already selected viewport enters the upstream global scratch store.
//! All access is serialized and cleared after each pane; no global logger,
//! tracing subscriber, file sink, mover thread, or independent input is installed.

use std::sync::Mutex;

use log::{Level, LevelFilter, Record};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::Block,
};
use tui_logger::{Drain, ExtLogRecord, LogFormatter, TuiLoggerWidget, TuiWidgetState};
use unicode_width::UnicodeWidthStr;

static PROJECTION: Mutex<()> = Mutex::new(());
const TARGET: &str = "yoctui.viewport";
const HOT_DEPTH: usize = 64;
const MAX_PROJECTION_BYTES: usize = 256 * 1024;

struct ViewportFormatter(Vec<Line<'static>>);

impl LogFormatter for ViewportFormatter {
    fn min_width(&self) -> u16 {
        1
    }

    fn format(&self, _width: usize, record: &ExtLogRecord) -> Vec<Line<'_>> {
        // `line` is a scratch row index, not an invented source-file location.
        vec![
            record
                .line
                .and_then(|index| self.0.get(index as usize))
                .cloned()
                .unwrap_or_default(),
        ]
    }
}

fn push_symbol(spans: &mut Vec<Span<'static>>, symbol: &str, style: Style) {
    if let Some(last) = spans.last_mut()
        && last.style == style
    {
        last.content.to_mut().push_str(symbol);
    } else {
        spans.push(Span::styled(symbol.to_owned(), style));
    }
}

fn carry_word(spans: &mut Vec<Span<'static>>, available: usize) -> Option<Vec<Span<'static>>> {
    let (index, byte, separator) = spans.iter().enumerate().rev().find_map(|(index, span)| {
        span.content
            .char_indices()
            .rev()
            .find(|(_, value)| value.is_whitespace())
            .map(|(byte, value)| (index, byte, value.len_utf8()))
    })?;
    if spans[..index]
        .iter()
        .all(|span| span.content.trim().is_empty())
        && spans[index].content[..byte].trim().is_empty()
    {
        return None;
    }
    let suffix_width = spans[index].content[byte + separator..].width()
        + spans[index + 1..].iter().map(Span::width).sum::<usize>();
    if suffix_width > available {
        return None;
    }
    let mut tail = spans.split_off(index + 1);
    let boundary = &mut spans[index];
    let suffix = boundary.content[byte + separator..].to_owned();
    if !suffix.is_empty() {
        tail.insert(0, Span::styled(suffix, boundary.style));
    }
    boundary.content.to_mut().truncate(byte);
    while let Some(last) = spans.last_mut() {
        let length = last.content.trim_end_matches(char::is_whitespace).len();
        last.content.to_mut().truncate(length);
        if !last.content.is_empty() {
            break;
        }
        spans.pop();
    }
    Some(tail)
}

fn ascii_line_bytes(line: &Line<'_>, width: usize, remaining: usize) -> Option<usize> {
    line.spans.iter().try_fold(0usize, |used, span| {
        let bytes = used.checked_add(span.content.len())?;
        (bytes <= width
            && bytes <= remaining
            && span
                .content
                .bytes()
                .all(|byte| (b' '..=b'~').contains(&byte)))
        .then_some(bytes)
    })
}

/// Project at most width × height cells, preserving graphemes and span styles.
/// Wrapping never cuts a wide or combining grapheme and never scans later rows.
fn viewport(text: Text<'_>, area: Rect, wrap: bool) -> Vec<Line<'static>> {
    let mut rows = Vec::new();
    if area.is_empty() {
        return rows;
    }
    let height = usize::from(area.height);
    let width = usize::from(area.width);
    let mut bytes = 0;
    for line in text.lines {
        let style = text.style.patch(line.style);
        // Profiled hot path: an already fitting printable ASCII row needs no
        // grapheme segmentation, per-character allocation, or wrapping pass.
        // Unicode, controls, long rows and exhausted byte budgets retain the
        // exact bounded grapheme path below.
        if let Some(length) = ascii_line_bytes(&line, width, MAX_PROJECTION_BYTES - bytes) {
            bytes += length;
            rows.push(
                Line::from(
                    line.spans
                        .into_iter()
                        .map(|span| {
                            Span::styled(span.content.into_owned(), style.patch(span.style))
                        })
                        .collect::<Vec<_>>(),
                )
                .style(text.style),
            );
            if rows.len() == height {
                break;
            }
            continue;
        }
        let mut spans = Vec::new();
        let mut used = 0;
        for grapheme in line.styled_graphemes(style) {
            bytes += grapheme.symbol.len();
            if bytes > MAX_PROJECTION_BYTES {
                rows.push(Line::raw("[viewport truncated]"));
                return rows;
            }
            let cells = grapheme.symbol.width();
            if cells > width {
                continue;
            }
            if used + cells > width {
                if !wrap {
                    break;
                }
                let whitespace = grapheme.symbol.chars().all(char::is_whitespace);
                let tail = if whitespace {
                    Vec::new()
                } else {
                    carry_word(&mut spans, width - cells).unwrap_or_default()
                };
                rows.push(Line::from(std::mem::take(&mut spans)).style(text.style));
                if rows.len() == height {
                    return rows;
                }
                used = tail.iter().map(Span::width).sum();
                spans = tail;
                if used == 0 && whitespace {
                    continue;
                }
            }
            push_symbol(&mut spans, grapheme.symbol, grapheme.style);
            used += cells;
        }
        rows.push(Line::from(spans).style(text.style));
        if rows.len() == height {
            break;
        }
    }
    rows
}

/// Keep the existing typed columns/header while delegating record display to
/// tui-logger. This does not reinterpret raw output as structured log fields.
pub(crate) fn table<'a>(
    frame: &mut Frame,
    area: Rect,
    block: Block<'_>,
    constraints: &[Constraint],
    headings: &[&str],
    rows: impl IntoIterator<Item = (Vec<Line<'a>>, Style)>,
) {
    let inner = block.inner(area);
    let columns = Layout::horizontal(constraints.iter().copied())
        .flex(Flex::Start)
        .spacing(1)
        .split(Rect::new(0, 0, inner.width, 1));
    let format_row = |cells: Vec<Line<'_>>, style: Style| {
        let mut spans = Vec::new();
        let mut end = 0;
        for (cell, column) in cells.into_iter().zip(columns.iter()) {
            spans.push(Span::raw(" ".repeat(column.x.saturating_sub(end) as usize)));
            let clipped = viewport(Text::from(cell), Rect::new(0, 0, column.width, 1), false);
            let width = clipped.first().map_or(0, Line::width) as u16;
            if let Some(line) = clipped.into_iter().next() {
                spans.extend(line.spans);
            }
            end = column.x.saturating_add(width);
        }
        spans.push(Span::raw(
            " ".repeat(inner.width.saturating_sub(end) as usize),
        ));
        Line::from(spans).style(style)
    };
    let mut lines = vec![format_row(
        headings.iter().map(|value| Line::raw(*value)).collect(),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    lines.extend(
        rows.into_iter()
            .take(inner.height.saturating_sub(1) as usize)
            .map(|(cells, style)| format_row(cells, style)),
    );
    render(frame, area, Text::from(lines), block, false);
}

struct ClearProjection;

impl Drop for ClearProjection {
    fn drop(&mut self) {
        tui_logger::move_events();
        tui_logger::set_buffer_depth(2);
    }
}

pub(crate) fn render(frame: &mut Frame, area: Rect, text: Text<'_>, block: Block<'_>, wrap: bool) {
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = viewport(text, inner, wrap);
    if lines.is_empty() {
        return;
    }
    // A fixed target prevents the upstream target registry from growing with
    // recipes, builds or pane identities. History and controls stay in App.
    let _lock = PROJECTION.lock().unwrap_or_else(|error| error.into_inner());
    let _clear = ClearProjection;
    tui_logger::move_events();
    tui_logger::set_buffer_depth(lines.len().max(2));
    tui_logger::set_hot_buffer_depth(HOT_DEPTH);
    let drain = Drain::new();
    for (index, line) in lines.iter().enumerate() {
        drain.log(
            &Record::builder()
                .level(Level::Info)
                .target(TARGET)
                .line(Some(index as u32))
                .args(format_args!("{line}"))
                .build(),
        );
        if (index + 1) % HOT_DEPTH == 0 {
            tui_logger::move_events();
        }
    }
    tui_logger::move_events();
    let state = TuiWidgetState::new().set_default_display_level(LevelFilter::Trace);
    frame.render_widget(
        TuiLoggerWidget::default()
            .state(&state)
            .formatter(Box::new(ViewportFormatter(lines))),
        inner,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        Terminal,
        backend::TestBackend,
        style::{Color, Modifier},
    };

    #[test]
    fn yocto_logger_ascii_fast_path_requires_complete_printable_bounded_lines() {
        let line = Line::from(vec![Span::raw("task: "), Span::raw("done")]);
        assert_eq!(ascii_line_bytes(&line, 10, 10), Some(10));
        assert_eq!(ascii_line_bytes(&line, 9, 10), None);
        assert_eq!(ascii_line_bytes(&line, 10, 9), None);
        for source in ["café", "界", "e\u{301}", "a\tb", "a\nb", "a\rb", "\u{7f}"] {
            assert_eq!(
                ascii_line_bytes(&Line::from(Span::raw(source)), 80, 80),
                None
            );
        }
        let mut terminal = Terminal::new(TestBackend::new(20, 2)).unwrap();
        terminal
            .draw(|frame| {
                render(
                    frame,
                    frame.area(),
                    Text::from(
                        Line::from(vec![
                            Span::styled("task: ", Style::default().fg(Color::Red)),
                            Span::styled(
                                "done",
                                Style::default().add_modifier(Modifier::UNDERLINED),
                            ),
                        ])
                        .style(Style::default().bg(Color::Blue)),
                    )
                    .style(Style::default().fg(Color::Green)),
                    Block::default(),
                    true,
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 0)].fg, Color::Red);
        assert_eq!(buffer[(6, 0)].fg, Color::Green);
        assert_eq!(buffer[(6, 0)].bg, Color::Blue);
        assert!(buffer[(6, 0)].modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn yocto_logger_viewport_is_bounded_unicode_safe_and_keeps_search_styles() {
        let style = Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::UNDERLINED);
        let rows = viewport(
            Text::from(vec![
                Line::from(vec![Span::styled(
                    "界e\u{301}界abcdef",
                    style
                )]);
                100_000
            ]),
            Rect::new(0, 0, 4, 3),
            true,
        );
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].to_string(), "界e\u{301}");
        assert!(rows.iter().all(|line| line.width() <= 4));
        assert!(
            rows.iter()
                .flat_map(|line| &line.spans)
                .all(|span| span.style == style)
        );
        assert!(viewport(Text::raw("hidden"), Rect::default(), true).is_empty());
    }

    #[test]
    fn yocto_logger_repeated_parallel_panes_do_not_duplicate_or_cross_contaminate() {
        std::thread::scope(|scope| {
            for owner in 0..4 {
                scope.spawn(move || {
                    let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
                    for generation in 0..40 {
                        let marker = format!("owner={owner} generation={generation}");
                        terminal
                            .draw(|frame| {
                                render(
                                    frame,
                                    frame.area(),
                                    Text::raw(marker.clone()),
                                    Block::default(),
                                    false,
                                );
                            })
                            .unwrap();
                        let buffer = terminal.backend().buffer();
                        let text = buffer
                            .content
                            .iter()
                            .map(|cell| cell.symbol())
                            .collect::<String>();
                        assert!(text.contains(&marker));
                        assert_eq!(text.matches("owner=").count(), 1);
                    }
                });
            }
        });
    }

    #[test]
    fn yocto_logger_more_than_hot_buffer_rows_keeps_all_visible_rows_and_no_color() {
        let mut terminal = Terminal::new(TestBackend::new(20, 150)).unwrap();
        terminal
            .draw(|frame| {
                let lines = (0..150)
                    .map(|index| Line::raw(format!("row {index:03}")))
                    .collect::<Vec<_>>();
                render(
                    frame,
                    frame.area(),
                    Text::from(lines),
                    Block::default(),
                    false,
                );
            })
            .unwrap();
        for row in 0..150 {
            let buffer = terminal.backend().buffer();
            let text = (0..20)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>();
            assert!(text.starts_with(&format!("row {row:03}")), "{row}: {text}");
            assert_eq!(buffer[(0, row)].fg, Color::Reset);
        }
    }

    #[test]
    fn yocto_logger_wrapping_preserves_log_words_and_existing_paragraph_layout() {
        use ratatui::{
            buffer::Buffer,
            widgets::{Paragraph, Widget, Wrap},
        };
        for source in [
            "retained 32 B/2 lines · dropped 4 B/1 lines · truncated 1 · hits 1",
            "Source: /home/user/yocto/build/tmp/work/busybox/temp/log.do_compile",
            "warning: compiler reports missing header file; review task output",
            "  indented code and café output",
        ] {
            for width in 12..80 {
                let area = Rect::new(0, 0, width, 12);
                let mut expected = Buffer::empty(area);
                Paragraph::new(source)
                    .wrap(Wrap { trim: false })
                    .render(area, &mut expected);
                let mut actual = Buffer::empty(area);
                Paragraph::new(viewport(Text::raw(source), area, true)).render(area, &mut actual);
                assert_eq!(actual, expected, "width={width}: {source}");
            }
        }
    }

    #[test]
    fn yocto_logger_wide_word_and_combining_payload_cannot_exceed_viewport_bounds() {
        for width in 2..10 {
            let rows = viewport(
                Text::raw("a 界界界 abc界def"),
                Rect::new(0, 0, width, 8),
                true,
            );
            assert!(rows.iter().all(|row| row.width() <= width as usize));
        }
        let source = format!("a{}", "\u{301}".repeat(MAX_PROJECTION_BYTES));
        let rows = viewport(Text::raw(source), Rect::new(0, 0, 24, 8), true);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].to_string(), "[viewport truncated]");
    }
}
