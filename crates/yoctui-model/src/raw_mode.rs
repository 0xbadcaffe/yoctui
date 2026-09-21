use crate::{
    CapabilityId, CapabilityState, DaemonCompatibilitySnapshot, PopupEditor, PopupEditorCommand,
    Recipe, RecipeMetadata,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};
use thiserror::Error;

include!("raw_mode/identity_and_parameters.rs");
include!("raw_mode/selectors.rs");
include!("raw_mode/argv.rs");
include!("raw_mode/preview.rs");
include!("raw_mode/output_retention.rs");
include!("raw_mode/execution_state.rs");
include!("raw_mode/execution_reduction.rs");
include!("raw_mode/view_state.rs");
include!("raw_mode/command_model.rs");
include!("raw_mode/favorites.rs");
include!("raw_mode/catalog.rs");
include!("raw_mode/state_accessors.rs");
include!("raw_mode/reducer_dispatch.rs");
include!("raw_mode/output_navigation.rs");
include!("raw_mode/form_navigation.rs");
include!("raw_mode/preview_and_favorites.rs");
include!("raw_mode/catalog_validation.rs");

#[cfg(test)]
#[path = "tests/raw_execution/mod.rs"]
mod raw_execution_tests;

#[cfg(test)]
#[path = "tests/raw_mode_state/mod.rs"]
mod raw_mode_state_tests;

#[cfg(test)]
#[path = "tests/raw_preview/mod.rs"]
mod raw_preview_tests;

#[cfg(test)]
#[path = "tests/raw_argv/mod.rs"]
mod raw_argv_tests;

#[cfg(test)]
#[path = "tests/raw_selector/mod.rs"]
mod raw_selector_tests;

#[cfg(test)]
#[path = "tests/raw_parameter/mod.rs"]
mod raw_parameter_tests;

#[cfg(test)]
#[path = "tests/raw_catalog_model/mod.rs"]
mod raw_catalog_model_tests;

#[cfg(test)]
#[path = "tests/raw_catalog_trace/mod.rs"]
mod raw_catalog_trace_tests;

#[cfg(test)]
#[path = "tests/raw_capability/mod.rs"]
mod raw_capability_tests;
