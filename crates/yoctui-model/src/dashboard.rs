//! Typed operational-dashboard projection over existing reducer authority.

use crate::*;
use std::{path::Path, time::SystemTime};

include!("dashboard/types.rs");
include!("dashboard/projections.rs");

#[cfg(test)]
#[path = "tests/dashboard/mod.rs"]
mod tests;
