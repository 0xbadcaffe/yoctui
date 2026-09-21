use crate::{
    App, BackgroundJobs, BuildEnvironmentState, BuildRecord, BuildState, CapabilityId,
    CapabilityImplementation, CapabilitySnapshot, CompletedTask, FocusTarget,
    ImageArtifactInventoryState, LogState, MaintenanceState, PackageDetailState, PackageIdentity,
    PackageInventoryState, ProjectProfileState, QaState, QemuCapability, QemuSession,
    RootfsCompositionState, Screen, SdkArtifactInventoryState, SdkSession, SdkToolCapability,
    SecurityState, SignatureComparisonState, SignatureDumpState, TaskId, TaskInfo, TestCapability,
    TestComparisonState, TestJunitExportState, TestResultInventoryState, TestSession, Theme,
    WicCapability, WicDeviceInventoryState, WicOutputInventoryState, WicSession, Workspace,
};
use std::collections::{BTreeMap, HashMap, VecDeque};
use thiserror::Error;

include!("daemon_state/global_state.rs");
include!("daemon_state/updates.rs");
include!("daemon_state/jobs.rs");
include!("daemon_state/client_view.rs");
include!("daemon_state/replica_and_errors.rs");
include!("daemon_state/bounded_collections.rs");

#[cfg(test)]
#[path = "tests/daemon_state/mod.rs"]
mod tests;
