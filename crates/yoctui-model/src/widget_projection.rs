//! Bounded, renderer-independent projections for shared visual widgets.
//!
//! These values contain no terminal geometry, Ratatui state, or domain parsing.
//! Reducers and typed adapters decide meaning; renderers only choose a bounded
//! presentation for that meaning.

use crate::BoundedScroll;
use std::collections::VecDeque;

include!("widget_projection/gauge.rs");
include!("widget_projection/collections.rs");

#[cfg(test)]
#[path = "tests/widget_projection/mod.rs"]
mod tests;
