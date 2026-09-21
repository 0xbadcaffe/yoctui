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
                        Span::styled("done", Style::default().add_modifier(Modifier::UNDERLINED)),
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
        "Source: /workspace/yocto/build/tmp/work/busybox/temp/log.do_compile",
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
