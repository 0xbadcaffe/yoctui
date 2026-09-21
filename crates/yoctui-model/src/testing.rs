use crate::{BackgroundJobId, BuildRequest};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use yoctui_utils::{is_absolute_normal_path_within, is_bounded_identifier, is_bounded_plain_text};

include!("testing/results.rs");
include!("testing/inventory_and_comparison.rs");
include!("testing/junit_and_capabilities.rs");
include!("testing/launch.rs");
include!("testing/sessions.rs");

#[cfg(test)]
#[path = "tests/testing/mod.rs"]
mod tests;
