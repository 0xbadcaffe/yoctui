use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

include!("compatibility/environment_identity.rs");
include!("compatibility/capability_ids.rs");
include!("compatibility/capability_state.rs");
include!("compatibility/identity_validation.rs");

#[cfg(test)]
#[path = "tests/compatibility_capability/mod.rs"]
mod capability;

#[cfg(test)]
#[path = "tests/compatibility_cache/mod.rs"]
mod compatibility_cache;

#[cfg(test)]
#[path = "tests/compatibility_environment/mod.rs"]
mod environment_identity;
