use crate::{
    CapabilityAvailabilitySummary, CapabilityEvidence, CapabilityId, CapabilityImplementation,
    CapabilityReason, CapabilityState, ClientReplicaStatus, CommandId, DaemonCompatibilitySnapshot,
    Dialog, Effect, EnvironmentOperatingMode, Screen, WorkspaceAvailability,
    WorkspaceAvailabilityState, WorkspaceCompatibilityState, WorkspaceEffectRequirement,
    YoctoEnvironmentIdentity, workspace_destination_requirement, workspace_dialog_requirement,
    workspace_effect_requirement, workspace_screen_destination,
};

include!("compatibility_ui/projection.rs");
include!("compatibility_ui/action_availability.rs");
include!("compatibility_ui/workspace_action_types.rs");
include!("compatibility_ui/workspace_action_catalog.rs");
include!("compatibility_ui/workspace_action_projection.rs");

#[cfg(test)]
#[path = "tests/compatibility_ui/mod.rs"]
mod tests;
