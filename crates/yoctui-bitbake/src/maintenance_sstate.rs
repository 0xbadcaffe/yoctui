use std::{
    collections::{BTreeMap, VecDeque},
    ffi::{OsStr, OsString},
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    MAX_MAINTENANCE_LIMITATIONS, MAX_MAINTENANCE_PATHS, MAX_MAINTENANCE_TEXT_BYTES,
    MaintenanceCapabilitySnapshot, MaintenanceFileIdentity, MaintenanceMetadata,
    MaintenanceOperation, MaintenanceOperationPreview, MaintenanceOutputStream,
    MaintenanceSessionId, MaintenanceTool, MaintenanceToolCapability, MaintenanceToolInterface,
    PrServiceOperation, PrServiceRequest, SstateCleanupMode, SstateCleanupPreview,
    SstateCleanupRequest, SstateReadinessMode, SstateReadinessRequest,
};
use yoctui_utils::is_transient_spawn_error;

const SSTATE_EVENT_CHANNEL_CAPACITY: usize = 64;
const SSTATE_OPERATION_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const MAX_PREVIEW_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const SPAWN_ATTEMPTS: usize = 4;
const SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

include!("maintenance_sstate/capability_inspection.rs");
include!("maintenance_sstate/filesystem_validation.rs");
include!("maintenance_sstate/command_types.rs");
include!("maintenance_sstate/command_planning.rs");
include!("maintenance_sstate/preview_and_events.rs");
include!("maintenance_sstate/job_runner.rs");

#[cfg(test)]
#[path = "tests/maintenance_sstate/mod.rs"]
mod tests;
