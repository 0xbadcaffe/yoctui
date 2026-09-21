use crate::Workspace;
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeSet, fmt};
use thiserror::Error;

include!("project_profile/profile_and_workflows.rs");
include!("project_profile/paths_and_items.rs");
include!("project_profile/validation.rs");

#[cfg(test)]
#[path = "tests/project_profile/mod.rs"]
mod tests;
