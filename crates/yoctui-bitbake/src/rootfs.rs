use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use serde::de::{Deserializer as _, IgnoredAny, MapAccess, Visitor};
use thiserror::Error;
use yoctui_model::{
    ImageArtifactIdentity, MAX_ROOTFS_DEPTH, MAX_ROOTFS_ENTRIES, MAX_ROOTFS_PACKAGES,
    MAX_ROOTFS_SYSTEM_PREVIEW_BYTES, PackageIdentity, RootfsAuthority, RootfsComposition,
    RootfsCompositionRequest, RootfsDbusService, RootfsEntry, RootfsEntryKind,
    RootfsFilesystemTree, RootfsInstalledPackage, RootfsPackageInventory, RootfsPathIdentity,
    RootfsSystemInventory, RootfsSystemdService,
};

const ROOTFS_SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PKGDATA_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PKGDATA_LINE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PKGDATA_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ROOTFS_ACCOUNTED_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
const MAX_LIMITATIONS: usize = 64;
const MAX_SYSTEM_RECORDS: usize = 4_096;
const MAX_SYSTEM_FILE_BYTES: u64 = 1024 * 1024;

mod udev;

include!("rootfs/types_and_adapter.rs");
include!("rootfs/source_and_system_scan.rs");
include!("rootfs/manifest_and_pkgdata.rs");
include!("rootfs/package_and_filesystem.rs");

#[cfg(test)]
#[path = "tests/rootfs/mod.rs"]
mod tests;
