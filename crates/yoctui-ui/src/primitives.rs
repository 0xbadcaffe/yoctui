//! Reusable, render-only workbench primitives.
//!
//! These helpers receive already-resolved text and styles. They do not inspect
//! backend data, mutate model state, or own workspace selection.

use ratatui::{
    prelude::{Line, Span, Style, Text},
    widgets::{Block, Borders, Paragraph, Row, Wrap},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneStyles {
    pub base: Style,
    pub border: Style,
    pub focused_border: Style,
    pub selected: Style,
    pub inactive_selected: Style,
    pub table_header: Style,
    pub muted: Style,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneShell<'a> {
    title: Line<'a>,
    focused: bool,
    styles: PaneStyles,
    borders: Borders,
}

impl<'a> PaneShell<'a> {
    pub fn new(title: impl Into<Line<'a>>, focused: bool, styles: PaneStyles) -> Self {
        Self {
            title: title.into(),
            focused,
            styles,
            borders: Borders::ALL,
        }
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    pub fn block(self) -> Block<'a> {
        Block::default()
            .title(self.title)
            .borders(self.borders)
            .style(self.styles.base)
            .border_style(if self.focused {
                self.styles.focused_border
            } else {
                self.styles.border
            })
    }

    pub fn row_style(&self, selected: bool, focus_owner: bool) -> Style {
        match (selected, focus_owner) {
            (true, true) => self.styles.selected,
            (true, false) => self.styles.inactive_selected,
            (false, _) => self.styles.base,
        }
    }

    pub fn table_header_style(&self) -> Style {
        self.styles.table_header
    }
}

pub fn section_header<'a>(
    title: impl Into<Span<'a>>,
    detail: Option<impl Into<Span<'a>>>,
    separator_style: Style,
) -> Line<'a> {
    let mut spans = vec![title.into()];
    if let Some(detail) = detail {
        spans.push(Span::styled("  ·  ", separator_style));
        spans.push(detail.into());
    }
    Line::from(spans)
}

pub fn separator(width: u16, style: Style) -> Line<'static> {
    Line::from(Span::styled("─".repeat(usize::from(width)), style))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Success,
    Warning,
    Error,
    Running,
    Pending,
    Accent,
    Muted,
    Info,
    Disabled,
}

impl StatusTone {
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Success => "✓",
            Self::Warning => "!",
            Self::Error => "✕",
            Self::Running => "▶",
            Self::Pending => "…",
            Self::Accent => "◆",
            Self::Muted => "·",
            Self::Info => "i",
            Self::Disabled => "–",
        }
    }
}

pub fn status_label<'a>(tone: StatusTone, label: impl Into<String>, style: Style) -> Span<'a> {
    Span::styled(format!("{} {}", tone.marker(), label.into()), style)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Empty,
    Loading,
    Unavailable,
    Partial,
    Error,
}

impl StateKind {
    const fn marker(self) -> &'static str {
        match self {
            Self::Empty => "∅",
            Self::Loading => "…",
            Self::Unavailable | Self::Partial => "!",
            Self::Error => "✕",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateView {
    pub kind: StateKind,
    pub summary: String,
    pub detail: Option<String>,
    pub action: Option<String>,
}

impl StateView {
    pub fn text(&self, summary_style: Style, detail_style: Style) -> Text<'static> {
        let mut lines = vec![Line::from(Span::styled(
            format!("{} {}", self.kind.marker(), self.summary),
            summary_style,
        ))];
        if let Some(detail) = self.detail.as_ref().filter(|detail| !detail.is_empty()) {
            lines.push(Line::from(Span::styled(detail.clone(), detail_style)));
        }
        if let Some(action) = self.action.as_ref().filter(|action| !action.is_empty()) {
            lines.push(Line::from(Span::styled(action.clone(), detail_style)));
        }
        Text::from(lines)
    }

    pub fn paragraph(&self, summary_style: Style, detail_style: Style) -> Paragraph<'static> {
        Paragraph::new(self.text(summary_style, detail_style)).wrap(Wrap { trim: false })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedScrollIndicator {
    pub offset: usize,
    pub viewport: usize,
    pub total: usize,
}

impl BoundedScrollIndicator {
    pub fn new(offset: usize, viewport: usize, total: usize) -> Self {
        let viewport = viewport.max(1);
        let maximum = total.saturating_sub(viewport);
        Self {
            offset: offset.min(maximum),
            viewport,
            total,
        }
    }

    pub fn is_scrollable(self) -> bool {
        self.total > self.viewport
    }

    pub fn label(self) -> String {
        if self.total == 0 {
            return "0/0".into();
        }
        let first = self.offset.saturating_add(1).min(self.total);
        let last = self
            .offset
            .saturating_add(self.viewport)
            .min(self.total)
            .max(first);
        let up = if self.offset > 0 { "↑" } else { " " };
        let down = if last < self.total { "↓" } else { " " };
        if first == last {
            format!("{up}{down} {first}/{}", self.total)
        } else {
            format!("{up}{down} {first}-{last}/{}", self.total)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponsiveColumn {
    pub minimum_width: u16,
    /// Lower values are more important. Priority zero is mandatory.
    pub priority: u8,
}

pub fn responsive_columns(width: u16, columns: &[ResponsiveColumn]) -> Vec<bool> {
    let mut visible = vec![false; columns.len()];
    let mut used = 0_u16;
    let highest_priority = columns
        .iter()
        .map(|column| column.priority)
        .max()
        .unwrap_or(0);
    for priority in 0..=highest_priority {
        for (index, column) in columns.iter().enumerate() {
            if column.priority != priority {
                continue;
            }
            if priority == 0 || used.saturating_add(column.minimum_width) <= width {
                visible[index] = true;
                used = used.saturating_add(column.minimum_width);
            }
        }
    }
    visible
}

pub fn selected_row<'a>(
    cells: impl IntoIterator<Item = ratatui::widgets::Cell<'a>>,
    selected: bool,
    focus_owner: bool,
    shell: &PaneShell<'_>,
) -> Row<'a> {
    Row::new(cells).style(shell.row_style(selected, focus_owner))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        Terminal,
        backend::TestBackend,
        prelude::{Color, Modifier},
        widgets::Paragraph,
    };

    fn styles() -> PaneStyles {
        PaneStyles {
            base: Style::default().fg(Color::White),
            border: Style::default().fg(Color::DarkGray),
            focused_border: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            selected: Style::default().bg(Color::Blue),
            inactive_selected: Style::default().add_modifier(Modifier::UNDERLINED),
            table_header: Style::default().add_modifier(Modifier::BOLD),
            muted: Style::default().add_modifier(Modifier::DIM),
        }
    }

    #[test]
    fn foundation_ui_primitives_render_focus_header_separator_and_states() {
        let styles = styles();
        let shell = PaneShell::new(
            section_header(
                Span::raw("Tasks"),
                Some(Span::styled("Running", Style::default().fg(Color::Green))),
                styles.muted,
            ),
            true,
            styles,
        );
        let state = StateView {
            kind: StateKind::Unavailable,
            summary: "Unavailable — CPU sample missing.".into(),
            detail: Some("Host did not publish this metric.".into()),
            action: None,
        };
        let mut terminal = Terminal::new(TestBackend::new(50, 8)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let inner = shell.clone().block().inner(area);
                frame.render_widget(shell.clone().block(), area);
                frame.render_widget(
                    state.paragraph(
                        Style::default().fg(Color::Yellow),
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                    inner,
                );
                frame.render_widget(Paragraph::new(separator(12, styles.muted)), inner);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("Tasks"), "{text}");
        assert!(text.contains("Running"), "{text}");
        assert_eq!(buffer[(0, 0)].fg, Color::Cyan);
    }

    #[test]
    fn foundation_ui_primitives_distinguish_active_and_inactive_selection() {
        let styles = styles();
        let shell = PaneShell::new("Rows", false, styles);
        assert_eq!(
            shell.row_style(true, true),
            Style::default().bg(Color::Blue)
        );
        assert!(
            shell
                .row_style(true, false)
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
        assert_eq!(shell.table_header_style(), styles.table_header);
    }

    #[test]
    fn foundation_ui_primitives_bound_scroll_indicators() {
        assert_eq!(BoundedScrollIndicator::new(99, 5, 12).label(), "↑  8-12/12");
        assert_eq!(BoundedScrollIndicator::new(0, 5, 12).label(), " ↓ 1-5/12");
        assert_eq!(BoundedScrollIndicator::new(0, 5, 0).label(), "0/0");
        assert!(!BoundedScrollIndicator::new(0, 20, 4).is_scrollable());
    }

    #[test]
    fn foundation_ui_primitives_hide_only_lower_priority_columns() {
        let columns = [
            ResponsiveColumn {
                minimum_width: 10,
                priority: 0,
            },
            ResponsiveColumn {
                minimum_width: 8,
                priority: 2,
            },
            ResponsiveColumn {
                minimum_width: 7,
                priority: 1,
            },
        ];
        assert_eq!(responsive_columns(17, &columns), vec![true, false, true]);
        assert_eq!(responsive_columns(25, &columns), vec![true, true, true]);
        assert_eq!(responsive_columns(4, &columns), vec![true, false, false]);
    }

    #[test]
    fn foundation_ui_primitives_statuses_have_text_markers() {
        for tone in [
            StatusTone::Success,
            StatusTone::Warning,
            StatusTone::Error,
            StatusTone::Running,
            StatusTone::Pending,
            StatusTone::Accent,
            StatusTone::Muted,
            StatusTone::Info,
            StatusTone::Disabled,
        ] {
            let label = status_label(tone, "state", Style::default());
            assert!(label.content.contains("state"));
            assert!(!tone.marker().is_empty());
        }
        for kind in [
            StateKind::Empty,
            StateKind::Loading,
            StateKind::Unavailable,
            StateKind::Partial,
            StateKind::Error,
        ] {
            assert!(!kind.marker().is_empty());
        }
    }
}
