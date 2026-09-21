use std::{collections::HashMap, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::{
    OperatorActionDefinition, OperatorActionId, OperatorActionTarget, WorkspaceDestination,
    global_operator_action_definitions,
};

include!("keymap/keystrokes.rs");
include!("keymap/preferences.rs");
include!("keymap/effective.rs");
include!("keymap/chord_and_ui.rs");
include!("keymap/validation.rs");

#[cfg(test)]
#[path = "tests/keymap/mod.rs"]
mod tests;
