use crate::{BackgroundJobId, BuildRequest};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use yoctui_utils::{is_absolute_normal_path_within, is_bounded_identifier};

include!("sdk/build_and_artifacts.rs");
include!("sdk/publish.rs");
include!("sdk/native_tools.rs");
include!("sdk/sessions.rs");

#[cfg(test)]
#[path = "tests/sdk/mod.rs"]
mod tests;
