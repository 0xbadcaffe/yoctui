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

