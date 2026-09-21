use crate::{CapabilityId, CapabilityReason};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

include!("compatibility_catalog/types.rs");
include!("compatibility_catalog/builtin.rs");
include!("compatibility_catalog/definition_helpers.rs");
include!("compatibility_catalog/definition_core.rs");
include!("compatibility_catalog/definition_raw.rs");
include!("compatibility_catalog/definition_development.rs");
include!("compatibility_catalog/definition_workflows.rs");
include!("compatibility_catalog/definitions.rs");
include!("compatibility_catalog/validation.rs");

#[cfg(test)]
#[path = "tests/compatibility_catalog/mod.rs"]
mod catalog;
