//! Safe, bounded persistence for reconstructable daemon metadata.
use crate::daemon::{
    BitBakeCapability, BitBakeState, DaemonInstanceId, DaemonSnapshot, JobSummary, LifecycleState,
    LogRecord, MAX_RAW_EXECUTION_REQUESTS, ProjectProfileSummary, PtyKind, RawAttachmentData,
    RawExecutionOutcomeData, RawExecutionPhaseData, RawExecutionResultData,
    RawExecutionSnapshotData, RawHistoryRecordData, TerminalDimensions, WorkspaceIdentity,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error;

pub const DAEMON_PERSIST_SCHEMA_VERSION: u32 = 1;
pub const MAX_DAEMON_PERSIST_BYTES: u64 = 4 * 1024 * 1024;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(1);

include!("daemon_persist/recovery_state.rs");
include!("daemon_persist/storage.rs");

#[cfg(test)]
#[path = "tests/daemon_persist/mod.rs"]
mod tests;
