use super::*;
use crate::terminal_render::TerminalReplicaAdapter;

#[test]
fn terminal_viewport_borrows_visible_cells_without_copying_strings_or_losing_style() {
    let screen = yoctui_model::ClientDaemonPtyScreen {
        session_id: 1,
        columns: 4,
        rows_count: 2,
        cursor_column: 2,
        cursor_row: 1,
        cursor_hidden: false,
        application_cursor: true,
        scrollback_offset: 0,
        scrollback_lines: 0,
        dropped_line_feeds_lower_bound: 0,
        rows: vec![],
        cells: "abcdefgh"
            .chars()
            .map(|ch| yoctui_model::ClientDaemonTerminalCell {
                contents: ch.to_string(),
                bold: true,
                foreground: yoctui_model::ClientDaemonTerminalColor::Rgb(10, 20, 30),
                ..Default::default()
            })
            .collect(),
    };
    let adapter = TerminalReplicaAdapter::new(&screen, 1, 1, true, 1, 2).unwrap();
    assert_eq!(adapter.cells.len(), 2);
    assert!(std::ptr::eq(adapter.cells[0].source, &screen.cells[5]));
    assert!(std::ptr::eq(adapter.cells[1].source, &screen.cells[6]));
    let mut cell = ratatui::buffer::Cell::default();
    tui_term::widget::Cell::apply(&adapter.cells[0], &mut cell);
    assert_eq!(cell.symbol(), "f");
    assert_eq!(cell.fg, Color::Rgb(10, 20, 30));
    assert!(cell.modifier.contains(Modifier::BOLD));
    assert_eq!(tui_term::widget::Screen::cursor_position(&adapter), (0, 1));
}
