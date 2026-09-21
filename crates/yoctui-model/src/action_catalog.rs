use std::collections::HashSet;

use crate::{
    CommandId, CompatibilityUiWorkspaceActionDefinition, WorkspaceDestination,
    WorkspaceEffectRequirement, compatibility_ui_command_action_definition,
    compatibility_ui_workspace_action_seeds,
};

include!("action_catalog/types.rs");
include!("action_catalog/global_metadata.rs");
include!("action_catalog/global_actions.rs");
include!("action_catalog/workspace_actions.rs");
include!("action_catalog/validation.rs");

#[cfg(test)]
#[path = "tests/action_catalog/mod.rs"]
mod tests;
