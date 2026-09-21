//! Typed hierarchical progress projection over existing reducer authority.

use crate::*;

include!("progress/types_and_helpers.rs");
include!("progress/hierarchy.rs");
include!("progress/background_jobs.rs");

#[cfg(test)]
#[path = "tests/progress/mod.rs"]
mod tests;
