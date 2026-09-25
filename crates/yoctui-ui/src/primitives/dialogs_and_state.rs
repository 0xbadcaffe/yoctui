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
            Self::Confirmation => Some("Confirmation"),
            Self::Destructive => Some("Warning"),
            Self::Result => Some("Result"),
            Self::Error => Some("Error"),
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
            Some(tone) => format!("{tone} · {}", self.title),
            None => self.title,
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

    /// Compact title chrome for a clipped, selection-owned collection.
    ///
    /// The cue is absent when every retained row fits. `selection` is a
    /// zero-based retained index; invalid values are clamped to the final row
    /// so rendering can never advertise a position outside the collection.
    pub fn title_label(
        self,
        selection: Option<usize>,
        detailed: bool,
        unicode: bool,
    ) -> Option<String> {
        if !self.is_scrollable() || self.total == 0 {
            return None;
        }
        let selected = selection
            .unwrap_or(self.offset)
            .min(self.total.saturating_sub(1))
            .saturating_add(1);
        let last = self.offset.saturating_add(self.viewport).min(self.total);
        let up = self.offset > 0;
        let down = last < self.total;
        let directions = match (up, down, unicode) {
            (true, true, true) => "↑↓",
            (true, false, true) => "↑",
            (false, true, true) => "↓",
            (true, true, false) => "^v",
            (true, false, false) => "^",
            (false, true, false) => "v",
            (false, false, _) => "",
        };
        if detailed {
            Some(format!(
                "{selected}/{} · {directions} · rows {}–{last}",
                self.total,
                self.offset.saturating_add(1)
            ))
        } else {
            Some(format!("{selected}/{} {directions}", self.total))
        }
    }
}
