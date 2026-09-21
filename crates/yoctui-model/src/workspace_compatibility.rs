use crate::{
    App, BuildRequest, CapabilityId, CapabilityState, DaemonCompatibilitySnapshot, Dialog, Effect,
    MaintenanceDialog, MaintenanceEffect, MaintenanceOperation, QaDialog, QaEffect,
    RawCapabilityRequirement, RawExecutionPolicy, Screen, SdkOperation, SecurityDialog,
    SecurityEffect, SecurityOperation, TestFamily, TestLaunchPreview, TestOperation, WicOperation,
    builtin_raw_catalog,
};
use thiserror::Error;

include!("workspace_compatibility/types.rs");
include!("workspace_compatibility/availability.rs");
include!("workspace_compatibility/destinations.rs");
include!("workspace_compatibility/effects.rs");
include!("workspace_compatibility/dialogs.rs");
include!("workspace_compatibility/authority.rs");

#[cfg(test)]
#[path = "tests/workspace_compatibility/mod.rs"]
mod tests;
