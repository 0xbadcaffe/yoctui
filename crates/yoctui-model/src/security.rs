use crate::{
    BackgroundJobId, BuildRequest, PopupEditor, RecipeIdentity, popup_toml_document,
    popup_toml_value,
};
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};
use yoctui_utils::{is_absolute_normal_path_within, is_bounded_identifier, is_bounded_plain_text};

include!("security/capabilities.rs");
include!("security/reports.rs");
include!("security/inventory_and_state.rs");
include!("security/actions_and_helpers.rs");
include!("security/update_capability.rs");
include!("security/update_session.rs");
include!("security/update_imports.rs");
include!("security/update_reports.rs");
include!("security/update_navigation.rs");
include!("security/update.rs");

#[cfg(test)]
#[path = "tests/security/mod.rs"]
mod tests;
