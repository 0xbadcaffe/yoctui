use crate::{
    BackgroundJobId, BuildRequest, PopupEditor, RecipeIdentity, popup_toml_document,
    popup_toml_value,
};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::SystemTime,
};
use yoctui_utils::{is_absolute_normal_path_within, is_bounded_identifier, is_bounded_plain_text};

include!("qa/capability_scopes.rs");
include!("qa/check_capabilities.rs");
include!("qa/sessions.rs");
include!("qa/reports.rs");
include!("qa/inventory_and_state.rs");
include!("qa/actions_and_helpers.rs");
include!("qa/update_recipe_capability.rs");
include!("qa/update_recipe_session.rs");
include!("qa/update_imports.rs");
include!("qa/update_reports.rs");
include!("qa/update_report_navigation.rs");
include!("qa/update_layer_capability.rs");
include!("qa/update_layer_session.rs");
include!("qa/update.rs");

#[cfg(test)]
#[path = "tests/qa/mod.rs"]
mod tests;
