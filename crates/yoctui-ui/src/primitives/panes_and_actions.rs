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
