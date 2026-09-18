//! Sdk build.
use super::*;

pub(crate) fn sdk_build_is_populate(request: &BuildRequest) -> bool {
    matches!(
        request.task.as_deref(),
        Some("populate_sdk" | "populate_sdk_ext")
    )
}

pub(crate) fn sdk_refresh_after_build_event(
    app: &mut App,
    pending_sdk_build: &mut Option<BuildRequest>,
    event: &BackendEvent,
) -> Option<Effect> {
    match event {
        BackendEvent::BuildCompleted { success: true, .. } => {
            let request = pending_sdk_build.take()?;
            sdk_build_is_populate(&request)
                .then(|| compatibility_workspace_action(app, Action::RefreshSdkArtifactInventory))
                .flatten()
        }
        BackendEvent::BuildCompleted { success: false, .. }
        | BackendEvent::CommandFailed { .. }
        | BackendEvent::Disconnected => {
            *pending_sdk_build = None;
            None
        }
        _ => None,
    }
}
