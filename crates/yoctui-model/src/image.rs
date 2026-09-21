use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

include!("image/artifacts.rs");
include!("image/normalization.rs");
include!("image/state.rs");

#[cfg(test)]
#[path = "tests/image/mod.rs"]
mod tests;
