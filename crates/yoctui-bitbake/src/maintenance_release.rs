use std::{
    collections::{BTreeMap, VecDeque},
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;
use yoctui_model::{
    BuildComparisonRequest, GitArchiveRequest, LockedSignatureCacheRequest,
    MAX_MAINTENANCE_ARGUMENTS, MAX_MAINTENANCE_EVIDENCE, MAX_MAINTENANCE_LIMITATIONS,
    MAX_MAINTENANCE_PATHS, MAX_MAINTENANCE_TEXT_BYTES, MaintenanceCapabilitySnapshot,
    MaintenanceEvidence, MaintenanceFileIdentity, MaintenanceMetadata, MaintenanceOperation,
    MaintenanceOperationPreview, MaintenanceSessionId, MaintenanceTool, MaintenanceToolCapability,
    MaintenanceToolInterface,
};

use crate::maintenance_sstate::{
    MaintenanceExternalCommand, MaintenanceFilesystemGuard, MaintenanceSstateAdapterError,
    MaintenanceSstateCommandKind, MaintenanceSstateCommandSpec, guard_directory,
    guard_directory_or_absent, guard_regular_file,
};

const RELEASE_OPERATION_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const MAX_EVIDENCE_SCAN_DIRECTORIES: usize = 4_096;

include!("maintenance_release/capability_and_commands.rs");
include!("maintenance_release/arguments_and_validation.rs");
include!("maintenance_release/evidence_snapshot.rs");

#[cfg(test)]
#[path = "tests/maintenance_release/mod.rs"]
mod tests;
