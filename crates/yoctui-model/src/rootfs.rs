use crate::{ImageArtifactIdentity, PackageIdentity};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

include!("rootfs/types.rs");
include!("rootfs/normalization.rs");
include!("rootfs/totals_and_groups.rs");
include!("rootfs/state.rs");

#[cfg(test)]
#[path = "tests/rootfs/mod.rs"]
mod tests;
