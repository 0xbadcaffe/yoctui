//! Bounded, renderer-independent projections for shared visual widgets.
//!
//! These values contain no terminal geometry, Ratatui state, or domain parsing.
//! Reducers and typed adapters decide meaning; renderers only choose a bounded
//! presentation for that meaning.

use crate::BoundedScroll;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetRole {
    Primary,
    Success,
    Warning,
    Error,
    Running,
    Pending,
    Disabled,
    Accent,
    Muted,
    Informational,
    Progress,
    Cpu,
    Memory,
    DiskRead,
    DiskWrite,
    NetworkRx,
    NetworkTx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    Available,
    Active,
    Empty,
    Unknown,
    Unavailable,
    Partial,
    TerminalSuccess,
    TerminalFailure,
    TerminalCancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetTerminalState {
    Success,
    Failure,
    Cancelled,
}

impl WidgetTerminalState {
    const fn widget_state(self) -> WidgetState {
        match self {
            Self::Success => WidgetState::TerminalSuccess,
            Self::Failure => WidgetState::TerminalFailure,
            Self::Cancelled => WidgetState::TerminalCancelled,
        }
    }

    const fn role(self) -> WidgetRole {
        match self {
            Self::Success => WidgetRole::Success,
            Self::Failure => WidgetRole::Error,
            Self::Cancelled => WidgetRole::Warning,
        }
    }
}

impl WidgetState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Active => "active",
            Self::Empty => "empty",
            Self::Unknown => "unknown",
            Self::Unavailable => "unavailable",
            Self::Partial => "partial",
            Self::TerminalSuccess => "succeeded",
            Self::TerminalFailure => "failed",
            Self::TerminalCancelled => "cancelled",
        }
    }

    pub const fn marker(self, unicode: bool, reduced_motion: bool) -> &'static str {
        match self {
            Self::Available => {
                if unicode {
                    "◆"
                } else {
                    "+"
                }
            }
            Self::Active if unicode && !reduced_motion => "…",
            Self::Active => ">",
            Self::Empty => {
                if unicode {
                    "∅"
                } else {
                    "-"
                }
            }
            Self::Unknown => "?",
            Self::Unavailable => "!",
            Self::Partial => "!",
            Self::TerminalSuccess => {
                if unicode {
                    "✓"
                } else {
                    "+"
                }
            }
            Self::TerminalFailure => {
                if unicode {
                    "✕"
                } else {
                    "x"
                }
            }
            Self::TerminalCancelled => {
                if unicode {
                    "■"
                } else {
                    "#"
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedFraction {
    pub current: u64,
    pub total: u64,
}

impl BoundedFraction {
    pub const fn new(current: u64, total: u64) -> Option<Self> {
        if total == 0 {
            None
        } else {
            Some(Self { current, total })
        }
    }

    pub fn percent(self) -> u8 {
        let percent = u128::from(self.current)
            .saturating_mul(100)
            .checked_div(u128::from(self.total))
            .unwrap_or(0)
            .min(100);
        u8::try_from(percent).unwrap_or(100)
    }

    pub fn ratio(self) -> f64 {
        (self.current.min(self.total) as f64 / self.total as f64).clamp(0.0, 1.0)
    }

    pub fn exact_text(self) -> String {
        format!("{}/{} ({}%)", self.current, self.total, self.percent())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GaugeProjection {
    pub label: String,
    pub state: WidgetState,
    pub role: WidgetRole,
    pub fraction: Option<BoundedFraction>,
    pub detail: Option<String>,
}

impl GaugeProjection {
    pub fn determinate(
        label: impl Into<String>,
        current: u64,
        total: u64,
        role: WidgetRole,
    ) -> Self {
        let label = label.into();
        match BoundedFraction::new(current, total) {
            Some(fraction) if current <= total => Self {
                label,
                state: WidgetState::Available,
                role,
                fraction: Some(fraction),
                detail: None,
            },
            Some(fraction) => Self {
                label,
                state: WidgetState::Partial,
                role: WidgetRole::Warning,
                fraction: Some(fraction),
                detail: Some("reported value exceeds total".into()),
            },
            None => Self::explicit(label, WidgetState::Unknown, role, "total not reported"),
        }
    }

    pub fn indeterminate(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::explicit(label, WidgetState::Active, WidgetRole::Running, detail)
    }

    pub fn explicit(
        label: impl Into<String>,
        state: WidgetState,
        role: WidgetRole,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            state,
            role,
            fraction: None,
            detail: Some(detail.into()).filter(|detail| !detail.is_empty()),
        }
    }

    pub fn terminal(
        label: impl Into<String>,
        current: u64,
        total: u64,
        terminal: WidgetTerminalState,
        detail: impl Into<String>,
    ) -> Self {
        let role = terminal.role();
        if total == 0 {
            let detail = detail.into();
            return Self {
                label: label.into(),
                state: terminal.widget_state(),
                role,
                fraction: None,
                detail: Some(if detail.is_empty() {
                    format!("{current}/?")
                } else {
                    format!("{current}/? · {detail}")
                }),
            };
        }
        let mut projection = Self::determinate(label, current, total, role);
        if projection.fraction.is_some() {
            projection.state = terminal.widget_state();
            projection.role = role;
            projection.detail = Some(detail.into()).filter(|detail| !detail.is_empty());
        }
        projection
    }

    pub fn text(&self, unicode: bool, reduced_motion: bool) -> String {
        let mut parts = vec![format!(
            "{} {}",
            self.state.marker(unicode, reduced_motion),
            self.label
        )];
        if self.state != WidgetState::Available {
            parts.push(self.state.label().into());
        }
        if let Some(fraction) = self.fraction {
            parts.push(fraction.exact_text());
        }
        if let Some(detail) = &self.detail {
            parts.push(detail.clone());
        }
        parts.join(" · ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryProjection {
    pub label: String,
    pub state: WidgetState,
    pub role: WidgetRole,
    pub current: Option<u64>,
    pub value_suffix: Option<String>,
    pub points: Vec<u64>,
    pub detail: Option<String>,
}

impl HistoryProjection {
    pub fn bounded(
        label: impl Into<String>,
        state: WidgetState,
        role: WidgetRole,
        current: Option<u64>,
        points: impl IntoIterator<Item = u64>,
        maximum_points: usize,
        detail: impl Into<String>,
    ) -> Self {
        let mut bounded = VecDeque::new();
        for point in points {
            if maximum_points == 0 {
                continue;
            }
            if bounded.len() == maximum_points {
                bounded.pop_front();
            }
            bounded.push_back(point);
        }
        let points = bounded.into_iter().collect::<Vec<_>>();
        let state = if state == WidgetState::Available && points.is_empty() && current.is_none() {
            WidgetState::Empty
        } else {
            state
        };
        Self {
            label: label.into(),
            state,
            role,
            current,
            value_suffix: None,
            points,
            detail: Some(detail.into()).filter(|detail| !detail.is_empty()),
        }
    }

    pub fn with_value_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.value_suffix = Some(suffix.into()).filter(|suffix| !suffix.is_empty());
        self
    }

    pub fn text(&self, unicode: bool, reduced_motion: bool) -> String {
        let current = self.current.map_or_else(
            || self.state.label().to_owned(),
            |value| {
                format!(
                    "{value}{}",
                    self.value_suffix.as_deref().unwrap_or_default()
                )
            },
        );
        let detail = self
            .detail
            .as_deref()
            .map_or(String::new(), |detail| format!(" · {detail}"));
        format!(
            "{} {} {current}{detail}",
            self.state.marker(unicode, reduced_motion),
            self.label
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarValue {
    pub label: String,
    pub value: u64,
    pub role: WidgetRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarProjection {
    pub state: WidgetState,
    pub values: Vec<BarValue>,
    pub detail: Option<String>,
}

impl BarProjection {
    pub fn bounded(
        state: WidgetState,
        values: impl IntoIterator<Item = BarValue>,
        maximum_values: usize,
        detail: impl Into<String>,
    ) -> Self {
        let values = values.into_iter().take(maximum_values).collect::<Vec<_>>();
        let state = if state == WidgetState::Available && values.is_empty() {
            WidgetState::Empty
        } else {
            state
        };
        Self {
            state,
            values,
            detail: Some(detail.into()).filter(|detail| !detail.is_empty()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabProjection {
    pub labels: Vec<String>,
    pub selected: Option<usize>,
}

impl TabProjection {
    pub fn bounded(
        labels: impl IntoIterator<Item = String>,
        selected: usize,
        maximum_tabs: usize,
    ) -> Self {
        let labels = labels.into_iter().take(maximum_tabs).collect::<Vec<_>>();
        let selected = (!labels.is_empty()).then(|| selected.min(labels.len() - 1));
        Self { labels, selected }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegendItem {
    pub label: String,
    pub value: String,
    pub role: WidgetRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegendProjection {
    pub state: WidgetState,
    pub items: Vec<LegendItem>,
    pub detail: Option<String>,
}

impl LegendProjection {
    pub fn bounded(
        state: WidgetState,
        items: impl IntoIterator<Item = LegendItem>,
        maximum_items: usize,
        detail: impl Into<String>,
    ) -> Self {
        let items = items.into_iter().take(maximum_items).collect::<Vec<_>>();
        let state = if state == WidgetState::Available && items.is_empty() {
            WidgetState::Empty
        } else {
            state
        };
        Self {
            state,
            items,
            detail: Some(detail.into()).filter(|detail| !detail.is_empty()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarProjection {
    pub scroll: BoundedScroll,
    pub state: WidgetState,
}

impl ScrollbarProjection {
    pub fn new(selection: usize, offset: usize, viewport: usize, total: usize) -> Self {
        Self {
            scroll: BoundedScroll::new(selection, offset, viewport.max(1), total),
            state: if total == 0 {
                WidgetState::Empty
            } else {
                WidgetState::Available
            },
        }
    }

    pub fn label(self) -> String {
        self.scroll.range_label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ux_widget_projection_preserves_exact_progress_and_exception_states() {
        let progress = GaugeProjection::determinate("Build", 7, 9, WidgetRole::Progress);
        assert_eq!(progress.fraction.unwrap().exact_text(), "7/9 (77%)");
        assert!(progress.text(true, false).contains("7/9 (77%)"));

        let invalid = GaugeProjection::determinate("Build", 10, 9, WidgetRole::Progress);
        assert_eq!(invalid.state, WidgetState::Partial);
        assert!(invalid.text(false, true).contains("10/9 (100%)"));

        let unknown = GaugeProjection::determinate("Build", 0, 0, WidgetRole::Progress);
        assert_eq!(unknown.state, WidgetState::Unknown);
        assert!(unknown.text(false, true).contains("total not reported"));
    }

    #[test]
    fn ux_widget_projection_bounds_large_histories_bars_tabs_and_legends() {
        let history = HistoryProjection::bounded(
            "CPU",
            WidgetState::Available,
            WidgetRole::Cpu,
            Some(999),
            0..10_000,
            60,
            "60-sample history",
        );
        assert_eq!(history.points.len(), 60);
        assert_eq!(history.points[0], 9_940);

        let bars = BarProjection::bounded(
            WidgetState::Available,
            (0..1_000).map(|value| BarValue {
                label: format!("包{value}"),
                value,
                role: WidgetRole::Accent,
            }),
            8,
            "",
        );
        assert_eq!(bars.values.len(), 8);

        let tabs = TabProjection::bounded(
            (0..1_000).map(|index| format!("Tab {index}")),
            usize::MAX,
            6,
        );
        assert_eq!(tabs.selected, Some(5));

        let legend = LegendProjection::bounded(
            WidgetState::Available,
            bars.values.iter().map(|bar| LegendItem {
                label: bar.label.clone(),
                value: bar.value.to_string(),
                role: bar.role,
            }),
            3,
            "",
        );
        assert_eq!(legend.items.len(), 3);
    }

    #[test]
    fn ux_widget_projection_keeps_empty_partial_terminal_and_scroll_text_explicit() {
        let empty = HistoryProjection::bounded(
            "I/O",
            WidgetState::Available,
            WidgetRole::DiskRead,
            None,
            [],
            60,
            "",
        );
        assert_eq!(empty.state, WidgetState::Empty);
        assert!(empty.text(false, true).contains("empty"));

        let terminal = GaugeProjection::terminal(
            "Build",
            8,
            10,
            WidgetTerminalState::Failure,
            "two tasks failed",
        );
        assert!(terminal.text(false, true).contains("x Build · failed"));

        let terminal_unknown = GaugeProjection::terminal(
            "Parse",
            12,
            0,
            WidgetTerminalState::Cancelled,
            "backend stopped",
        );
        assert_eq!(terminal_unknown.state, WidgetState::TerminalCancelled);
        assert!(terminal_unknown.text(false, true).contains("12/?"));

        let scroll = ScrollbarProjection::new(999, 999, 4, 12);
        assert_eq!(scroll.label(), "9-12/12");
        assert_eq!(
            ScrollbarProjection::new(0, 0, 5, 0).state,
            WidgetState::Empty
        );
    }

    #[test]
    fn ux_accessibility_every_widget_state_has_ascii_marker_label_and_exact_text() {
        for state in [
            WidgetState::Available,
            WidgetState::Active,
            WidgetState::Empty,
            WidgetState::Unknown,
            WidgetState::Unavailable,
            WidgetState::Partial,
            WidgetState::TerminalSuccess,
            WidgetState::TerminalFailure,
            WidgetState::TerminalCancelled,
        ] {
            assert!(state.marker(false, true).is_ascii());
            let projection = GaugeProjection::explicit(
                "Build progress",
                state,
                WidgetRole::Progress,
                "3/10 authoritative tasks",
            );
            let text = projection.text(false, true);
            assert!(text.contains("Build progress"), "{text}");
            if state != WidgetState::Available {
                assert!(text.contains(state.label()), "{text}");
            }
            assert!(text.contains("3/10 authoritative tasks"), "{text}");
        }
        let exact = GaugeProjection::determinate("Build", 3, 10, WidgetRole::Progress);
        assert!(exact.text(false, true).contains("3/10 (30%)"));
    }
}
