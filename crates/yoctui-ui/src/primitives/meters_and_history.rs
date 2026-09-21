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
        render_segmented_dot_meter(
            frame,
            area,
            fraction.ratio(),
            &label,
            style,
            styles.muted,
            options.unicode,
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
        render_segmented_dot_meter(
            frame,
            area,
            fraction.ratio(),
            &label,
            style,
            styles.muted,
            options.unicode,
        );
    } else {
        frame.render_widget(Paragraph::new(label).style(style), area);
    }
}

fn render_segmented_dot_meter(
    frame: &mut ratatui::Frame,
    area: Rect,
    ratio: f64,
    label: &str,
    filled_style: Style,
    muted_style: Style,
    unicode: bool,
) {
    let filled = (ratio.clamp(0.0, 1.0) * f64::from(area.width)).ceil() as usize;
    for index in 0..usize::from(area.width) {
        let Some(cell) = frame
            .buffer_mut()
            .cell_mut((area.x.saturating_add(index as u16), area.y))
        else {
            continue;
        };
        if index < filled {
            cell.set_symbol(if unicode { "▪" } else { "#" })
                .set_style(filled_style);
        } else {
            cell.set_symbol(if unicode { "▫" } else { "." })
                .set_style(muted_style);
        }
    }
    frame.render_widget(
        Paragraph::new(label)
            .style(filled_style)
            .alignment(ratatui::prelude::Alignment::Center),
        area,
    );
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
