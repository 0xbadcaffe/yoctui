use std::path::{Path, PathBuf};

include!("wic/capability.rs");
include!("wic/create.rs");
include!("wic/output_inventory.rs");
include!("wic/devices_and_write.rs");
include!("wic/sessions_and_normalization.rs");

#[cfg(test)]
#[path = "tests/wic/mod.rs"]
mod tests;
