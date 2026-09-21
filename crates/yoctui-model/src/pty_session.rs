use std::{
    collections::BTreeSet,
    path::{Component, PathBuf},
};

use thiserror::Error;

include!("pty_session/types.rs");
include!("pty_session/state_machine.rs");
include!("pty_session/validation.rs");

#[cfg(test)]
#[path = "tests/pty_session/mod.rs"]
mod tests;
