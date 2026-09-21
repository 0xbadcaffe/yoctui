use crate::{PopupEditor, Screen, popup_toml_fields};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::SystemTime,
};
use yoctui_utils::is_absolute_normal_path_within;

include!("maintenance/capabilities.rs");
include!("maintenance/services.rs");
include!("maintenance/integrations.rs");
include!("maintenance/operations.rs");
include!("maintenance/sstate_and_service_drafts.rs");
include!("maintenance/release_drafts.rs");
include!("maintenance/state.rs");
include!("maintenance/transition_helpers.rs");
include!("maintenance/update_inspection.rs");
include!("maintenance/update_readiness.rs");
include!("maintenance/update_cleanup.rs");
include!("maintenance/update_pr_service.rs");
include!("maintenance/update_locked_cache.rs");
include!("maintenance/update_build_history.rs");
include!("maintenance/update_git_archive.rs");
include!("maintenance/update_operation_confirmation.rs");
include!("maintenance/update_session_lifecycle.rs");
include!("maintenance/update.rs");

#[cfg(test)]
#[path = "tests/maintenance/mod.rs"]
mod tests;
