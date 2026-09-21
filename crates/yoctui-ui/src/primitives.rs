//! Reusable, render-only workbench primitives.
//!
//! These helpers receive already-resolved text and styles. They do not inspect
//! backend data, mutate model state, or own workspace selection.

use ratatui::{
    prelude::{Constraint, Layout, Line, Modifier, Rect, Span, Style, Text},
    symbols,
    widgets::{
        Bar, BarChart, Block, Borders, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Sparkline, Tabs, Wrap,
    },
};
use yoctui_model::{
    BarProjection, GaugeProjection, HistoryProjection, LegendProjection, ScrollbarProjection,
    TabProjection, WidgetRole, WidgetState,
};

include!("primitives/panes_and_actions.rs");

include!("primitives/dialogs_and_state.rs");

include!("primitives/meters_and_history.rs");

include!("primitives/charts_and_navigation.rs");

#[cfg(test)]
#[path = "tests/primitives/mod.rs"]
mod tests;
