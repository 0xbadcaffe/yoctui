//! Reusable, render-only workbench primitives.
//!
//! These helpers receive already-resolved text and styles. They do not inspect
//! backend data, mutate model state, or own workspace selection.

use ratatui::{
    prelude::{Constraint, Layout, Line, Modifier, Rect, Span, Style, Text},
    symbols,
    widgets::{
        Bar, BarChart, Block, Borders, Gauge, LineGauge, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Sparkline, Tabs, Wrap,
    },
};
use yoctui_model::{
    BarProjection, GaugeProjection, HistoryProjection, LegendProjection, ScrollbarProjection,
    TabProjection, WidgetRole, WidgetState,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionListItem {
    pub marker: &'static str,
    pub label: String,
    pub shortcut: String,
    pub state: String,
    pub enabled: bool,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionListStyles {
    pub enabled: Style,
    pub disabled: Style,
    pub shortcut: Style,
    pub detail: Style,
}

fn action_list_widths(items: &[ActionListItem], width: u16) -> (usize, bool) {
    let expanded = width >= 36;
    let shortcut_width = items
        .iter()
        .map(|item| item.shortcut.chars().count())
        .max()
        .unwrap_or(1);
    let maximum_label = items
        .iter()
        .map(|item| item.label.chars().count())
        .max()
        .unwrap_or(1);
    let reserved = shortcut_width.saturating_add(8);
    let available_label = usize::from(width).saturating_sub(reserved).max(8);
    (maximum_label.min(available_label), expanded)
}

fn clipped_label(label: &str, width: usize) -> String {
    let count = label.chars().count();
    if count <= width {
        return label.to_owned();
    }
    if width <= 1 {
        return "…".into();
    }
    format!("{}…", label.chars().take(width - 1).collect::<String>())
}

pub fn action_list_plain(items: &[ActionListItem], width: u16) -> String {
    let (label_width, expanded) = action_list_widths(items, width);
    items
        .iter()
        .flat_map(|item| {
            let label = clipped_label(&item.label, label_width);
            let row = if expanded {
                format!(
                    "{} {label:<label_width$}  [{}] — {}",
                    item.marker, item.shortcut, item.state
                )
            } else {
                format!(
                    "{} {label} [{}] — {}",
                    item.marker, item.shortcut, item.state
                )
            };
            std::iter::once(row).chain(item.details.iter().map(|detail| format!("  {detail}")))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn action_list(
    items: &[ActionListItem],
    width: u16,
    styles: ActionListStyles,
) -> Text<'static> {
    let (label_width, expanded) = action_list_widths(items, width);
    let mut lines = Vec::new();
    for item in items {
        let item_style = if item.enabled {
            styles.enabled
        } else {
            styles.disabled
        };
        let shortcut_style = if item.enabled {
            styles.shortcut
        } else {
            styles.disabled
        };
        let label = clipped_label(&item.label, label_width);
        let spans = if expanded {
            vec![
                Span::styled(
                    format!("{} {label:<label_width$}  ", item.marker),
                    item_style,
                ),
                Span::styled(format!("[{}]", item.shortcut), shortcut_style),
                Span::styled(format!(" — {}", item.state), item_style),
            ]
        } else {
            vec![
                Span::styled(format!("{} {label} [", item.marker), item_style),
                Span::styled(item.shortcut.clone(), shortcut_style),
                Span::styled(format!("] — {}", item.state), item_style),
            ]
        };
        lines.push(Line::from(spans));
        lines.extend(item.details.iter().map(|detail| {
            Line::styled(
                format!("  {detail}"),
                if item.enabled {
                    styles.detail
                } else {
                    styles.disabled
                },
            )
        }));
    }
    Text::from(lines)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogTone {
    Standard,
    Confirmation,
    Destructive,
    Result,
    Error,
}

impl DialogTone {
    const fn label(self) -> Option<&'static str> {
        match self {
            Self::Standard => None,
            Self::Confirmation => Some("confirm"),
            Self::Destructive => Some("destructive"),
            Self::Result => Some("result"),
            Self::Error => Some("error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogStyles {
    pub base: Style,
    pub focused_border: Style,
    pub heading: Style,
    pub selected: Style,
    pub disabled: Style,
    pub validation: Style,
    pub hint: Style,
    pub destructive: Style,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogShell {
    title: String,
    tone: DialogTone,
    styles: DialogStyles,
}

impl DialogShell {
    pub fn new(title: impl Into<String>, tone: DialogTone, styles: DialogStyles) -> Self {
        Self {
            title: title.into(),
            tone,
            styles,
        }
    }

    pub fn block(self) -> Block<'static> {
        let title = match self.tone.label() {
            Some(tone) => format!("{tone} modal · {}", self.title),
            None => format!("modal · {}", self.title),
        };
        let title_style = match self.tone {
            DialogTone::Destructive | DialogTone::Error => self.styles.destructive,
            _ => self.styles.heading,
        };
        Block::default()
            .title(Line::styled(title, title_style))
            .borders(Borders::ALL)
            .style(self.styles.base)
            .border_style(self.styles.focused_border)
    }

    pub fn field(
        &self,
        label: &str,
        value: impl Into<String>,
        label_width: usize,
        selected: bool,
        disabled: bool,
    ) -> Line<'static> {
        let marker = if disabled {
            "–"
        } else if selected {
            "▶"
        } else {
            " "
        };
        let style = if disabled {
            self.styles.disabled
        } else if selected {
            self.styles.selected
        } else {
            self.styles.base
        };
        Line::styled(
            format!("{marker} {label:<label_width$}: {}", value.into()),
            style,
        )
    }

    pub fn validation(&self, error: Option<&str>) -> Line<'static> {
        match error {
            Some(error) => Line::styled(format!("✕ Validation: {error}"), self.styles.validation),
            None => Line::styled("✓ Validation: ready", self.styles.hint),
        }
    }

    pub fn controls(
        &self,
        primary: Option<(&str, &str)>,
        secondary: &[(&str, &str)],
    ) -> Line<'static> {
        let mut spans = Vec::new();
        if let Some((key, label)) = primary {
            let style = if self.tone == DialogTone::Destructive {
                self.styles.destructive
            } else {
                self.styles.selected
            };
            spans.push(Span::styled(format!("[{key}] {label}"), style));
        }
        for (key, label) in secondary {
            if !spans.is_empty() {
                spans.push(Span::styled("  ", self.styles.hint));
            }
            spans.push(Span::styled(format!("[{key}] {label}"), self.styles.hint));
        }
        Line::from(spans)
    }
}

pub fn bounded_dialog_rect(area: Rect, preferred_width: u16, preferred_height: u16) -> Rect {
    let width = preferred_width.min(area.width.saturating_sub(2)).max(1);
    let height = preferred_height.min(area.height.saturating_sub(2)).max(1);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
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
        let bounded = yoctui_model::BoundedScroll::new(offset, offset, viewport.max(1), total);
        Self {
            offset: bounded.offset,
            viewport: bounded.viewport,
            total: bounded.total,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetStyles {
    pub primary: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub running: Style,
    pub pending: Style,
    pub disabled: Style,
    pub accent: Style,
    pub muted: Style,
    pub informational: Style,
    pub progress: Style,
    pub graph_cpu: Style,
    pub graph_memory: Style,
    pub graph_disk_read: Style,
    pub graph_disk_write: Style,
    pub graph_network_rx: Style,
    pub graph_network_tx: Style,
    pub selected: Style,
}

impl WidgetStyles {
    pub fn role(self, role: WidgetRole) -> Style {
        match role {
            WidgetRole::Primary => self.primary,
            WidgetRole::Success => self.success,
            WidgetRole::Warning => self.warning,
            WidgetRole::Error => self.error,
            WidgetRole::Running => self.running,
            WidgetRole::Pending => self.pending,
            WidgetRole::Disabled => self.disabled,
            WidgetRole::Accent => self.accent,
            WidgetRole::Muted => self.muted,
            WidgetRole::Informational => self.informational,
            WidgetRole::Progress => self.progress,
            WidgetRole::Cpu => self.graph_cpu,
            WidgetRole::Memory => self.graph_memory,
            WidgetRole::DiskRead => self.graph_disk_read,
            WidgetRole::DiskWrite => self.graph_disk_write,
            WidgetRole::NetworkRx => self.graph_network_rx,
            WidgetRole::NetworkTx => self.graph_network_tx,
        }
    }

    pub fn state(self, state: WidgetState, role: WidgetRole) -> Style {
        match state {
            WidgetState::Available => self.role(role),
            WidgetState::Active => self.running,
            WidgetState::Empty | WidgetState::Unknown | WidgetState::Unavailable => self.disabled,
            WidgetState::Partial | WidgetState::TerminalCancelled => self.warning,
            WidgetState::TerminalSuccess => self.success,
            WidgetState::TerminalFailure => self.error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetRenderOptions {
    pub unicode: bool,
    pub reduced_motion: bool,
}

impl Default for WidgetRenderOptions {
    fn default() -> Self {
        Self {
            unicode: true,
            reduced_motion: false,
        }
    }
}

fn bounded_line_text(value: &str, width: u16, unicode: bool) -> String {
    if Line::from(value).width() <= usize::from(width) {
        return value.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let suffix = if unicode { "…" } else { "~" };
    let suffix_width = if width == 1 {
        1
    } else {
        Line::from(suffix).width()
    };
    let budget = usize::from(width).saturating_sub(suffix_width);
    let mut output = String::new();
    for character in value.chars() {
        output.push(character);
        if Line::from(output.as_str()).width() > budget {
            output.pop();
            break;
        }
    }
    output.push_str(suffix);
    output
}

pub fn render_semantic_gauge(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &GaugeProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    let label = bounded_line_text(
        &projection.text(options.unicode, options.reduced_motion),
        area.width,
        options.unicode,
    );
    let style = styles.state(projection.state, projection.role);
    if let Some(fraction) = projection.fraction {
        frame.render_widget(
            Gauge::default()
                .ratio(fraction.ratio())
                .label(label)
                .use_unicode(options.unicode)
                .gauge_style(style),
            area,
        );
    } else {
        frame.render_widget(Paragraph::new(label).style(style), area);
    }
}

pub fn render_semantic_meter(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &GaugeProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    let label = bounded_line_text(
        &projection.text(options.unicode, options.reduced_motion),
        area.width,
        options.unicode,
    );
    let style = styles.state(projection.state, projection.role);
    if let Some(fraction) = projection.fraction {
        let (filled, unfilled) = if options.unicode {
            ("━", "─")
        } else {
            ("=", "-")
        };
        frame.render_widget(
            LineGauge::default()
                .ratio(fraction.ratio())
                .label(label)
                .filled_symbol(filled)
                .unfilled_symbol(unfilled)
                .filled_style(style)
                .unfilled_style(styles.muted),
            area,
        );
    } else {
        frame.render_widget(Paragraph::new(label).style(style), area);
    }
}

fn ascii_history(points: &[u64], width: u16) -> String {
    if width == 0 || points.is_empty() {
        return String::new();
    }
    let points = &points[points.len().saturating_sub(usize::from(width))..];
    let maximum = points.iter().copied().max().unwrap_or(0);
    const LEVELS: &[u8] = b"._-:=+*#@";
    points
        .iter()
        .map(|value| {
            if maximum == 0 {
                '.'
            } else {
                let index = (u128::from(*value) * (LEVELS.len() - 1) as u128 / u128::from(maximum))
                    as usize;
                char::from(LEVELS[index])
            }
        })
        .collect()
}

pub fn render_history_chart(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &HistoryProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    let style = styles.state(projection.state, projection.role);
    let text = bounded_line_text(
        &projection.text(options.unicode, options.reduced_motion),
        area.width,
        options.unicode,
    );
    if projection.points.is_empty() || area.height == 1 {
        frame.render_widget(Paragraph::new(text).style(style), area);
        return;
    }
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    frame.render_widget(Paragraph::new(text).style(style), rows[0]);
    if options.unicode {
        frame.render_widget(
            Sparkline::default()
                .data(&projection.points)
                .max(projection.points.iter().copied().max().unwrap_or(1).max(1))
                .style(style),
            rows[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(ascii_history(&projection.points, rows[1].width)).style(style),
            rows[1],
        );
    }
}

fn explicit_state_text(
    state: WidgetState,
    detail: Option<&str>,
    options: WidgetRenderOptions,
) -> String {
    let detail = detail.map_or(String::new(), |detail| format!(" · {detail}"));
    format!(
        "{} {}{detail}",
        state.marker(options.unicode, options.reduced_motion),
        state.label()
    )
}

fn bounded_explicit_state_text(
    state: WidgetState,
    detail: Option<&str>,
    options: WidgetRenderOptions,
    width: u16,
) -> String {
    let full = explicit_state_text(state, detail, options);
    if Line::from(full.as_str()).width() <= usize::from(width) {
        return full;
    }
    if Line::from(state.label()).width() <= usize::from(width) {
        return state.label().into();
    }
    bounded_line_text(state.label(), width, options.unicode)
}

pub fn render_bar_chart(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &BarProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    if projection.values.is_empty() || projection.state != WidgetState::Available {
        let text = bounded_explicit_state_text(
            projection.state,
            projection.detail.as_deref(),
            options,
            area.width,
        );
        frame.render_widget(
            Paragraph::new(text).style(styles.state(projection.state, WidgetRole::Primary)),
            area,
        );
        return;
    }
    if !options.unicode || area.height < 3 {
        let lines = projection
            .values
            .iter()
            .take(usize::from(area.height))
            .map(|bar| {
                Line::styled(
                    bounded_line_text(
                        &format!("{}: {}", bar.label, bar.value),
                        area.width,
                        options.unicode,
                    ),
                    styles.role(bar.role),
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), area);
        return;
    }
    let bars = projection
        .values
        .iter()
        .map(|bar| {
            Bar::with_label(bar.label.clone(), bar.value)
                .style(styles.role(bar.role))
                .value_style(styles.role(bar.role).add_modifier(Modifier::BOLD))
                .text_value(bar.value.to_string())
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        BarChart::new(bars)
            .bar_width(3)
            .bar_gap(1)
            .bar_set(symbols::bar::NINE_LEVELS)
            .value_style(styles.primary)
            .label_style(styles.primary),
        area,
    );
}

pub fn render_tabs(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &TabProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    if projection.labels.is_empty() {
        frame.render_widget(
            Paragraph::new(explicit_state_text(WidgetState::Empty, None, options))
                .style(styles.disabled),
            area,
        );
        return;
    }
    let titles = projection
        .labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            if Some(index) == projection.selected {
                format!("[{label}]")
            } else {
                label.clone()
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Tabs::new(titles)
            .select(projection.selected)
            .divider(if options.unicode { "│" } else { "|" })
            .style(styles.primary)
            .highlight_style(styles.selected.add_modifier(Modifier::BOLD)),
        area,
    );
}

pub fn render_legend(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &LegendProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    if projection.items.is_empty() || projection.state != WidgetState::Available {
        let text = bounded_explicit_state_text(
            projection.state,
            projection.detail.as_deref(),
            options,
            area.width,
        );
        frame.render_widget(
            Paragraph::new(text).style(styles.state(projection.state, WidgetRole::Primary)),
            area,
        );
        return;
    }
    let marker = if options.unicode { "■" } else { "#" };
    let lines = projection
        .items
        .iter()
        .take(usize::from(area.height))
        .map(|item| {
            Line::from(vec![
                Span::styled(format!("{marker} "), styles.role(item.role)),
                Span::styled(
                    bounded_line_text(
                        &format!("{}: {}", item.label, item.value),
                        area.width.saturating_sub(2),
                        options.unicode,
                    ),
                    styles.primary,
                ),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn render_scrollbar(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: ScrollbarProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    let label = projection.label();
    if projection.state == WidgetState::Empty || area.height == 1 {
        frame.render_widget(
            Paragraph::new(bounded_line_text(&label, area.width, options.unicode)).style(
                if projection.state == WidgetState::Empty {
                    styles.disabled
                } else {
                    styles.primary
                },
            ),
            area,
        );
        return;
    }
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    let mut state = ScrollbarState::new(projection.scroll.total)
        .position(projection.scroll.offset)
        .viewport_content_length(projection.scroll.viewport);
    let scrollbar = if options.unicode {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
    } else {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("#")
            .track_symbol(Some("|"))
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"))
    };
    frame.render_stateful_widget(scrollbar.style(styles.accent), rows[0], &mut state);
    frame.render_widget(
        Paragraph::new(bounded_line_text(&label, rows[1].width, options.unicode))
            .style(styles.primary),
        rows[1],
    );
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

    #[test]
    fn action_lists_align_shortcuts_and_keep_disabled_reasons_in_text() {
        let items = vec![
            ActionListItem {
                marker: "✓",
                label: "Open Logs".into(),
                shortcut: "l".into(),
                state: "Local".into(),
                enabled: true,
                details: Vec::new(),
            },
            ActionListItem {
                marker: "×",
                label: "Cancel active build".into(),
                shortcut: "c".into(),
                state: "Disabled".into(),
                enabled: false,
                details: vec!["Reason: No active build can be cancelled.".into()],
            },
        ];
        let plain = action_list_plain(&items, 48);
        assert!(
            plain.contains("Open Logs            [l] — Local"),
            "{plain}"
        );
        assert!(
            plain.contains("Cancel active build  [c] — Disabled"),
            "{plain}"
        );
        assert!(
            plain.contains("Reason: No active build can be cancelled."),
            "{plain}"
        );

        let styled = action_list(
            &items,
            32,
            ActionListStyles {
                enabled: Style::default(),
                disabled: Style::default().add_modifier(Modifier::DIM),
                shortcut: Style::default().add_modifier(Modifier::BOLD),
                detail: Style::default(),
            },
        );
        assert!(
            styled.lines[1].spans[0]
                .style
                .add_modifier
                .contains(Modifier::DIM)
        );
        assert!(
            styled
                .lines
                .iter()
                .any(|line| line.to_string().contains("[c]"))
        );
    }

    #[test]
    fn dialog_primitives_bound_geometry_and_keep_state_textual() {
        let dialog_styles = DialogStyles {
            base: Style::default().fg(Color::White),
            focused_border: Style::default().fg(Color::Cyan),
            heading: Style::default().add_modifier(Modifier::BOLD),
            selected: Style::default().bg(Color::Blue),
            disabled: Style::default().add_modifier(Modifier::DIM),
            validation: Style::default().fg(Color::Red),
            hint: Style::default().fg(Color::DarkGray),
            destructive: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        };
        let shell = DialogShell::new("Remove workspace", DialogTone::Destructive, dialog_styles);
        let field = shell.field("Recipe", "busybox", 8, true, false);
        let disabled = shell.field("Deploy", "unavailable", 8, false, true);
        let validation = shell.validation(Some("target changed"));
        let controls = shell.controls(Some(("Enter", "Confirm destructive")), &[("Esc", "Cancel")]);
        assert!(field.to_string().contains("▶ Recipe"));
        assert!(disabled.to_string().contains("– Deploy"));
        assert!(
            validation
                .to_string()
                .contains("✕ Validation: target changed")
        );
        assert!(controls.to_string().contains("[Enter] Confirm destructive"));
        assert!(controls.to_string().contains("[Esc] Cancel"));
        assert_eq!(
            bounded_dialog_rect(Rect::new(7, 11, 80, 24), 110, 30),
            Rect::new(8, 12, 78, 22)
        );

        let mut terminal = Terminal::new(TestBackend::new(50, 8)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(shell.block(), frame.area()))
            .unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("destructive modal · Remove workspace"));
        assert_eq!(terminal.backend().buffer()[(0, 0)].fg, Color::Cyan);
    }

    fn widget_styles() -> WidgetStyles {
        WidgetStyles {
            primary: Style::default().fg(Color::White),
            success: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            warning: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            error: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            running: Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
            pending: Style::default().fg(Color::Yellow),
            disabled: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
            accent: Style::default().fg(Color::Magenta),
            muted: Style::default().fg(Color::Gray),
            informational: Style::default().fg(Color::Cyan),
            progress: Style::default().fg(Color::Green),
            graph_cpu: Style::default().fg(Color::Cyan),
            graph_memory: Style::default().fg(Color::Magenta),
            graph_disk_read: Style::default().fg(Color::Blue),
            graph_disk_write: Style::default().fg(Color::LightBlue),
            graph_network_rx: Style::default().fg(Color::Green),
            graph_network_tx: Style::default().fg(Color::Yellow),
            selected: Style::default().add_modifier(Modifier::REVERSED),
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn ux_widget_primitives_render_semantic_numeric_visual_vocabulary() {
        let styles = widget_styles();
        let options = WidgetRenderOptions::default();
        let gauge = GaugeProjection::determinate("Build", 7, 9, WidgetRole::Progress);
        let meter = GaugeProjection::terminal(
            "Parse",
            4,
            4,
            yoctui_model::WidgetTerminalState::Success,
            "complete",
        );
        let history = HistoryProjection::bounded(
            "CPU",
            WidgetState::Partial,
            WidgetRole::Cpu,
            Some(71),
            [10, 20, 31, 40, 55, 71],
            60,
            "sample gap",
        );
        let bars = BarProjection::bounded(
            WidgetState::Available,
            [
                yoctui_model::BarValue {
                    label: "Packages".into(),
                    value: 32,
                    role: WidgetRole::Accent,
                },
                yoctui_model::BarValue {
                    label: "Files".into(),
                    value: 19,
                    role: WidgetRole::Informational,
                },
            ],
            8,
            "",
        );
        let tabs =
            TabProjection::bounded(["Summary".into(), "History".into(), "Details".into()], 1, 8);
        let legend = LegendProjection::bounded(
            WidgetState::Available,
            [
                yoctui_model::LegendItem {
                    label: "Runtime".into(),
                    value: "32 MiB".into(),
                    role: WidgetRole::Success,
                },
                yoctui_model::LegendItem {
                    label: "Debug".into(),
                    value: "19 MiB".into(),
                    role: WidgetRole::Warning,
                },
            ],
            8,
            "",
        );
        let scroll = ScrollbarProjection::new(11, 8, 4, 12);

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| {
                let rows = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(4),
                    Constraint::Length(6),
                    Constraint::Length(1),
                    Constraint::Length(4),
                    Constraint::Min(4),
                ])
                .split(frame.area());
                render_semantic_gauge(frame, rows[0], &gauge, styles, options);
                render_semantic_meter(frame, rows[1], &meter, styles, options);
                render_history_chart(frame, rows[2], &history, styles, options);
                render_bar_chart(frame, rows[3], &bars, styles, options);
                render_tabs(frame, rows[4], &tabs, styles, options);
                render_legend(frame, rows[5], &legend, styles, options);
                render_scrollbar(frame, rows[6], scroll, styles, options);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("7/9 (77%)"), "{text}");
        assert!(text.contains("4/4 (100%)"), "{text}");
        assert!(text.contains("CPU 71 · sample gap"), "{text}");
        assert!(text.contains("[History]"), "{text}");
        assert!(text.contains("Runtime: 32 MiB"), "{text}");
        assert!(text.contains("9-12/12"), "{text}");
        assert!(!text.contains('�'), "{text}");
    }

    #[test]
    fn ux_widget_primitives_ascii_no_color_and_reduced_motion_keep_text() {
        let attribute = Style::default().add_modifier(Modifier::BOLD);
        let styles = WidgetStyles {
            primary: Style::default(),
            success: attribute,
            warning: attribute,
            error: attribute.add_modifier(Modifier::UNDERLINED),
            running: attribute,
            pending: Style::default(),
            disabled: Style::default().add_modifier(Modifier::DIM),
            accent: attribute,
            muted: Style::default(),
            informational: attribute,
            progress: attribute,
            graph_cpu: attribute,
            graph_memory: attribute,
            graph_disk_read: attribute,
            graph_disk_write: attribute,
            graph_network_rx: attribute,
            graph_network_tx: attribute,
            selected: Style::default().add_modifier(Modifier::REVERSED),
        };
        let options = WidgetRenderOptions {
            unicode: false,
            reduced_motion: true,
        };
        let active = GaugeProjection::indeterminate("Runqueue progress unknown", "waiting");
        let unavailable = HistoryProjection::bounded(
            "Network",
            WidgetState::Unavailable,
            WidgetRole::NetworkRx,
            None,
            [],
            60,
            "host source missing",
        );
        let bars = BarProjection::bounded(
            WidgetState::Available,
            [yoctui_model::BarValue {
                label: "Unicode 包".into(),
                value: u64::MAX,
                role: WidgetRole::Accent,
            }],
            4,
            "",
        );
        let tabs = TabProjection::bounded(["One".into(), "Two".into()], usize::MAX, 4);

        let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
        terminal
            .draw(|frame| {
                let rows = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ])
                .split(frame.area());
                render_semantic_gauge(frame, rows[0], &active, styles, options);
                render_history_chart(frame, rows[1], &unavailable, styles, options);
                render_bar_chart(frame, rows[2], &bars, styles, options);
                render_tabs(frame, rows[3], &tabs, styles, options);
                render_scrollbar(
                    frame,
                    rows[4],
                    ScrollbarProjection::new(0, 0, 4, 0),
                    styles,
                    options,
                );
                render_semantic_meter(frame, Rect::default(), &active, styles, options);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(
            text.contains("> Runqueue progress unknown · active"),
            "{text}"
        );
        assert!(text.contains("! Network unavailable"), "{text}");
        assert!(text.contains("Unicode 包"), "{text}");
        assert!(text.contains("18446744073709551615"), "{text}");
        assert!(text.contains("[Two]"), "{text}");
        assert!(text.contains("0/0"), "{text}");
        assert!(!text.contains('…'), "{text}");
        assert!(!text.contains('█'), "{text}");
        assert!(!text.contains('�'), "{text}");
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset)
        );
    }

    #[test]
    fn ux_widget_primitives_explicit_empty_partial_and_failure_never_panic_narrow() {
        let styles = widget_styles();
        let options = WidgetRenderOptions::default();
        let partial = BarProjection::bounded(WidgetState::Partial, [], 10, "2 records omitted");
        let failed =
            LegendProjection::bounded(WidgetState::TerminalFailure, [], 10, "adapter failed");
        let empty_tabs = TabProjection::bounded([], usize::MAX, usize::MAX);
        let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
        terminal
            .draw(|frame| {
                let rows = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(frame.area());
                render_bar_chart(frame, rows[0], &partial, styles, options);
                render_legend(frame, rows[1], &failed, styles, options);
                render_tabs(frame, rows[2], &empty_tabs, styles, options);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("partial"), "{text}");
        assert!(text.contains("failed"), "{text}");
        assert!(text.contains("empty"), "{text}");
        assert!(!text.contains('�'), "{text}");
    }

    #[test]
    fn ux_widget_primitives_resolve_every_role_through_the_semantic_theme() {
        let theme = crate::SemanticTheme::for_theme(yoctui_model::Theme::HighContrast, true);
        let styles = theme.widget_styles();
        assert_eq!(styles.progress.fg, Some(theme.progress));
        assert_eq!(styles.graph_cpu.fg, Some(theme.graph_cpu));
        assert_eq!(styles.graph_memory.fg, Some(theme.graph_memory));
        assert_eq!(styles.graph_disk_read.fg, Some(theme.graph_disk_read));
        assert_eq!(styles.graph_disk_write.fg, Some(theme.graph_disk_write));
        assert_eq!(styles.graph_network_rx.fg, Some(theme.graph_network_rx));
        assert_eq!(styles.graph_network_tx.fg, Some(theme.graph_network_tx));

        let no_color =
            crate::SemanticTheme::for_theme(yoctui_model::Theme::DarkPro, false).widget_styles();
        for style in [
            no_color.success,
            no_color.warning,
            no_color.error,
            no_color.running,
            no_color.pending,
            no_color.progress,
            no_color.graph_cpu,
            no_color.graph_memory,
            no_color.graph_disk_read,
            no_color.graph_disk_write,
            no_color.graph_network_rx,
            no_color.graph_network_tx,
        ] {
            assert_eq!(style.fg, Some(Color::Reset));
            assert_ne!(style.add_modifier, Modifier::empty());
        }
    }
}
