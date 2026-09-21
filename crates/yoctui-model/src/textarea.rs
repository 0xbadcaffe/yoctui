use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

include!("textarea/types.rs");
include!("textarea/state_and_selection.rs");
include!("textarea/editing_and_navigation.rs");
include!("textarea/history_and_search.rs");
include!("textarea/validation_layout_and_save.rs");
include!("textarea/external_edits_and_internals.rs");
include!("textarea/text_algorithms.rs");

#[cfg(test)]
#[path = "tests/textarea/mod.rs"]
mod tests;
