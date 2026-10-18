use super::*;

#[test]
fn source_viewport_fast_path_preserves_wrapping_unicode_and_multiline_styles() {
    use ratatui::widgets::Widget;
    let app = App::new(32, 8192);
    let content = "/* comment\ncontinued */\n\n工程 e\u{301} 👩‍💻\n\tvalue\r\n".to_owned()
        + &"long words wrap here ".repeat(20)
        + "\nlast line\n";
    for file in [
        "test.dts",
        "test.rs",
        "test.bb",
        "test.rules",
        "test.txt",
        "test.md",
    ] {
        for width in [1, 4, 12, 80, 120] {
            for scroll in [0, 1, 2, 4, 5, 10, 100] {
                let area = Rect::new(0, 0, width, 8);
                let mut expected = Buffer::empty(area);
                let mut actual = expected.clone();
                Paragraph::new(source_preview(&content, file, &app))
                    .wrap(Wrap { trim: false })
                    .scroll((scroll, 0))
                    .render(area, &mut expected);
                let (text, remainder) =
                    scrolled_source_preview(&content, file, &app, width, scroll);
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .scroll((remainder, 0))
                    .render(area, &mut actual);
                assert_eq!(actual, expected, "{file}, width {width}, scroll {scroll}");
            }
        }
    }
}

#[test]
fn source_viewport_skips_only_rows_known_not_to_wrap() {
    let app = App::new(32, 8192);
    let content = "short\n".repeat(4096);
    let (text, scroll) = scrolled_source_preview(&content, "file.txt", &app, 80, 4000);
    assert_eq!(scroll, 0);
    assert_eq!(text.lines.len(), 96);
    let (_, scroll) =
        scrolled_source_preview("short\nlong wrapped line\ntail", "file.txt", &app, 5, 3);
    assert_eq!(scroll, 2);
    let (_, scroll) = scrolled_source_preview(&content, "file.txt", &app, 0, 12);
    assert_eq!(scroll, 12);
}

#[test]
fn metadata_rows_borrow_unfiltered_inventory_without_scanning() {
    let values: Vec<_> = (0..4096).collect();
    let rows = MetadataRows::new(&values, "", |_, _| panic!("empty query must not scan"));
    assert_eq!(rows.len(), values.len());
    assert_eq!(rows.ordinal(4095), Some(4095));
    assert_eq!(rows.ordinal(4096), None);
    assert_eq!(rows.get(4095), Some((4095, &4095)));
    assert!(rows.get(4096).is_none());
}

#[test]
fn metadata_rows_preserve_unicode_filter_ordinals_and_updated_inventories() {
    let mut values = vec!["Kernel", "工程Σ", "recipe", "KERNEL-debug"];
    for query in ["", "KERNEL", "工程Σ", "Σ", "none", " "] {
        let rows = MetadataRows::new(&values, query, |value, query| {
            matches_folded_metadata(query, &[value])
        });
        let expected: Vec<_> = values
            .iter()
            .enumerate()
            .filter(|(_, value)| matches_metadata(query, &[value]))
            .collect();
        assert_eq!(rows.len(), expected.len());
        for (ordinal, (original, value)) in expected.into_iter().enumerate() {
            assert_eq!(rows.get(ordinal), Some((original, value)));
            assert_eq!(rows.ordinal(original), Some(ordinal));
        }
        assert!(rows.get(rows.len()).is_none());
    }
    values.clear();
    let rows = MetadataRows::new(&values, "", |_, _| true);
    assert_eq!(rows.len(), 0);
    assert!(rows.ordinal(0).is_none());
    assert!(rows.get(0).is_none());
}

#[test]
fn contiguous_frame_style_matches_block_for_attributes_unicode_and_clipped_areas() {
    let bounds = Rect::new(5, 7, 8, 4);
    for area in [
        bounds,
        Rect::new(0, 0, 80, 24),
        Rect::new(7, 8, 2, 2),
        Rect::new(0, 0, 2, 2),
        Rect::new(5, 7, 0, 0),
    ] {
        for style in [
            Style::default(),
            Style::default().fg(Color::Cyan).bg(Color::Black),
            Style::default()
                .add_modifier(Modifier::ITALIC)
                .remove_modifier(Modifier::BOLD),
        ] {
            let mut expected = Buffer::empty(bounds);
            expected[(5, 7)]
                .set_symbol("工程")
                .set_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
            let mut actual = expected.clone();
            ratatui::widgets::Widget::render(Block::default().style(style), area, &mut expected);
            paint_frame_style(&mut actual, area, style);
            assert_eq!(actual, expected);
        }
    }
}
