//! Terminal render.
use super::*;

pub(crate) const WIDE_WORKBENCH_MIN_WIDTH: u16 = 130;
pub(crate) const LITERAL_REFERENCE_WIDTH: u16 = 160;

/// Terminal-safe frame from throbber-widgets-tui's BRAILLE_EIGHT_DOUBLE set.
/// The CLI uses the same visual language while waiting for daemon readiness.
pub fn startup_activity_symbol(phase: usize) -> &'static str {
    let symbols = throbber_widgets_tui::BRAILLE_EIGHT_DOUBLE.symbols;
    symbols[phase % symbols.len()]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchNavigation {
    Results,
    Matches,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchExit {
    Done,
    Close,
}

pub(crate) struct TerminalReplicaCell {
    pub(crate) source: yoctui_model::ClientDaemonTerminalCell,
    pub(crate) color_enabled: bool,
}

impl tui_term::widget::Cell for TerminalReplicaCell {
    fn has_contents(&self) -> bool {
        !self.source.contents.is_empty() && !self.source.wide_continuation
    }

    fn apply(&self, cell: &mut ratatui::buffer::Cell) {
        if self.has_contents() {
            cell.set_symbol(&self.source.contents);
        }
        let mut modifiers = Modifier::empty();
        if self.source.bold {
            modifiers |= Modifier::BOLD;
        }
        if self.source.dim {
            modifiers |= Modifier::DIM;
        }
        if self.source.italic {
            modifiers |= Modifier::ITALIC;
        }
        if self.source.underline {
            modifiers |= Modifier::UNDERLINED;
        }
        if self.source.inverse {
            modifiers |= Modifier::REVERSED;
        }
        let color = |value| {
            if !self.color_enabled {
                return Color::Reset;
            }
            match value {
                yoctui_model::ClientDaemonTerminalColor::Default => Color::Reset,
                yoctui_model::ClientDaemonTerminalColor::Indexed(index) => Color::Indexed(index),
                yoctui_model::ClientDaemonTerminalColor::Rgb(red, green, blue) => {
                    Color::Rgb(red, green, blue)
                }
            }
        };
        cell.set_style(
            Style::reset()
                .fg(color(self.source.foreground))
                .bg(color(self.source.background))
                .add_modifier(modifiers),
        );
    }
}

pub(crate) struct TerminalReplicaAdapter {
    pub(crate) columns: u16,
    pub(crate) rows: u16,
    pub(crate) cursor_column: u16,
    pub(crate) cursor_row: u16,
    pub(crate) cursor_hidden: bool,
    pub(crate) scrollback_offset: u32,
    pub(crate) cells: Vec<TerminalReplicaCell>,
    pub(crate) row_offset: u16,
    pub(crate) column_offset: u16,
}

impl TerminalReplicaAdapter {
    pub(crate) fn new(
        screen: &yoctui_model::ClientDaemonPtyScreen,
        row_offset: usize,
        column_offset: usize,
        color_enabled: bool,
        viewport_rows: u16,
        viewport_columns: u16,
    ) -> Option<Self> {
        let capacity = usize::from(screen.rows_count) * usize::from(screen.columns);
        if screen.cells.len() != capacity {
            return None;
        }
        let row_offset = u16::try_from(row_offset)
            .unwrap_or(u16::MAX)
            .min(screen.rows_count);
        let column_offset = u16::try_from(column_offset)
            .unwrap_or(u16::MAX)
            .min(screen.columns);
        let rows = screen
            .rows_count
            .saturating_sub(row_offset)
            .min(viewport_rows);
        let columns = screen
            .columns
            .saturating_sub(column_offset)
            .min(viewport_columns);
        let mut cells = Vec::with_capacity(usize::from(rows) * usize::from(columns));
        for row in row_offset..row_offset.saturating_add(rows) {
            let start = usize::from(row) * usize::from(screen.columns) + usize::from(column_offset);
            cells.extend(
                screen.cells[start..start + usize::from(columns)]
                    .iter()
                    .cloned()
                    .map(|source| TerminalReplicaCell {
                        source,
                        color_enabled,
                    }),
            );
        }
        Some(Self {
            columns,
            rows,
            cursor_column: screen.cursor_column,
            cursor_row: screen.cursor_row,
            cursor_hidden: screen.cursor_hidden,
            scrollback_offset: screen.scrollback_offset,
            cells,
            row_offset,
            column_offset,
        })
    }
}

impl tui_term::widget::Screen for TerminalReplicaAdapter {
    type C = TerminalReplicaCell;

    fn cell(&self, row: u16, column: u16) -> Option<&Self::C> {
        if row >= self.rows || column >= self.columns {
            return None;
        }
        let index = usize::from(row) * usize::from(self.columns) + usize::from(column);
        self.cells.get(index)
    }

    fn hide_cursor(&self) -> bool {
        self.cursor_hidden
    }

    fn cursor_position(&self) -> (u16, u16) {
        let row = u32::from(self.cursor_row)
            .saturating_add(self.scrollback_offset)
            .checked_sub(u32::from(self.row_offset));
        let column = self.cursor_column.checked_sub(self.column_offset);
        (
            row.and_then(|value| u16::try_from(value).ok())
                .unwrap_or(u16::MAX),
            column.unwrap_or(u16::MAX),
        )
    }
}

pub(crate) fn render_terminal_replica_content(
    frame: &mut Frame,
    app: &App,
    screen: &yoctui_model::ClientDaemonPtyScreen,
    area: Rect,
    row_offset: usize,
    column_offset: usize,
) -> bool {
    let Some(adapter) = TerminalReplicaAdapter::new(
        screen,
        row_offset,
        column_offset,
        app.color_enabled,
        area.height,
        area.width,
    ) else {
        return false;
    };
    let blank_cursor = if app.color_enabled {
        Style::default().fg(ThemePalette::for_app(app).focused_border)
    } else {
        Style::default().add_modifier(Modifier::REVERSED)
    };
    let cursor = tui_term::widget::Cursor::default()
        .style(blank_cursor)
        .overlay_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_widget(PseudoTerminal::new(&adapter).cursor(cursor), area);
    true
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn search_line(
    app: &App,
    query: &str,
    editing: bool,
    selected_index: Option<usize>,
    total: usize,
    navigation: SearchNavigation,
    exit: SearchExit,
    width: u16,
) -> Line<'static> {
    let mode = if editing {
        "EDITING"
    } else if query.is_empty() {
        "IDLE"
    } else {
        "FILTERED"
    };
    let current = if total == 0 {
        0
    } else {
        selected_index.unwrap_or(0).min(total.saturating_sub(1)) + 1
    };
    let navigation = match navigation {
        SearchNavigation::Results => "↑/↓ results",
        SearchNavigation::Matches => "n/N next/previous",
    };
    let exit = match exit {
        SearchExit::Done => {
            if editing {
                "Enter/Esc done"
            } else {
                "/ edit"
            }
        }
        SearchExit::Close => "Esc close",
    };
    let clear = if query.is_empty() {
        ""
    } else {
        " · Ctrl+U clear"
    };
    let query_budget = match width {
        130.. => width.saturating_sub(94).clamp(8, 36),
        60..=129 => width.saturating_sub(43).clamp(6, 24),
        _ => width.saturating_sub(20).clamp(4, 16),
    };
    let query = if query.is_empty() { "<empty>" } else { query };
    let query = bounded_cell_text(query, query_budget.saturating_sub(u16::from(editing)));
    let cursor = if editing { "▏" } else { "" };
    let text = if width >= 130 {
        format!(
            "/ Search [{mode}] · Query: {query}{cursor} · Results: {current}/{total} · {navigation}{clear} · {exit}"
        )
    } else if width >= 60 {
        let navigation = match navigation {
            "↑/↓ results" => "↑/↓",
            _ => "n/N",
        };
        format!("/ [{mode}] Query: {query}{cursor} · {current}/{total} · {navigation}{clear}")
    } else {
        format!("/ [{mode}] {current}/{total} {query}{cursor}")
    };
    let palette = ThemePalette::for_app(app);
    let style = if editing {
        palette.role(palette.accent, Modifier::BOLD)
    } else if query == "<empty>" {
        palette.role(palette.muted, Modifier::DIM)
    } else {
        palette.role(palette.informational, Modifier::BOLD)
    };
    Line::styled(bounded_cell_text(&text, width), style)
}
