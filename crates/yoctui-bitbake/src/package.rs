use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::Notify,
};
use yoctui_model::{
    CapabilityId, DaemonCompatibilitySnapshot, MAX_PACKAGE_LIMITATIONS, MAX_PACKAGE_RECORDS,
    PackageDetail, PackageDetailRequest, PackageField, PackageIdentity, PackageInventoryRequest,
    PackageSummary, normalize_package_detail, normalize_package_summaries,
};

const MAX_PACKAGE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_PACKAGE_OUTPUT_LINES: usize = 32_768;
const PACKAGE_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const PACKAGE_ARGUMENT_BATCH: usize = 128;
pub const PKGDATA_LIST_PACKAGES_IMPLEMENTATION: &str = "pkgdata.list_packages.argv";
pub const PKGDATA_PACKAGE_INFO_IMPLEMENTATION: &str = "pkgdata.package_info.argv";
pub const PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION: &str = "pkgdata.list_package_files.argv";
pub const PKGDATA_READ_VALUE_IMPLEMENTATION: &str = "pkgdata.read_value.argv";

include!("package/types_and_cancellation.rs");
include!("package/adapter.rs");
include!("package/response_parsing.rs");
include!("package/process_io.rs");

#[cfg(test)]
#[path = "tests/package/mod.rs"]
mod tests;
