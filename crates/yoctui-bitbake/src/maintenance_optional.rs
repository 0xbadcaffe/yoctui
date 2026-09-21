use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{
    MAX_MAINTENANCE_LIMITATIONS, MAX_MAINTENANCE_OUTPUT, MAX_MAINTENANCE_PATHS,
    MAX_MAINTENANCE_TEXT_BYTES, MaintenanceCapabilitySnapshot, MaintenanceFileIdentity,
    MaintenanceIntegrationsSnapshot, MaintenanceMetadata, MaintenanceTool,
    MaintenanceToolCapability, MaintenanceToolInterface, ServiceProcessEvidence,
};
pub use yoctui_model::{
    MaintenanceDirectoryIdentity, MaintenanceGitWorktreeIdentity, OptionalErrorReportIntegration,
    OptionalIntegrationState, OptionalPullRequestIntegration, OptionalRepoManifestIntegration,
    OptionalToasterIntegration,
};

const MAX_PROCESS_ENTRIES: usize = 4_096;
const MAX_PROCESS_BYTES: usize = 512;

include!("maintenance_optional/capability_inspection.rs");
include!("maintenance_optional/tool_discovery.rs");
include!("maintenance_optional/identity_validation.rs");

#[cfg(test)]
#[path = "tests/maintenance_optional/mod.rs"]
mod tests;
