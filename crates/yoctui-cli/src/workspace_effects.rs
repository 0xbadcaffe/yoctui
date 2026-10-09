//! Workspace effects.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalWorkspaceEffectRoute {
    ImageArtifacts,
    RootfsComposition,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DaemonDevtoolModifyCompletion {
    Pending,
    Succeeded,
    Failed,
}

pub(crate) fn daemon_devtool_modify_completion(
    app: &App,
    identity: &RecipeIdentity,
    known_jobs: &[u64],
) -> DaemonDevtoolModifyCompletion {
    daemon_devtool_completion_after(app, identity, known_jobs)
}

pub(crate) fn daemon_devtool_completion_after(
    app: &App,
    identity: &RecipeIdentity,
    known_jobs: &[u64],
) -> DaemonDevtoolModifyCompletion {
    let label = format!("Devtool {}", identity.name);
    let Some(job) = app.daemon.jobs.iter().rev().find(|job| {
        job.kind == yoctui_model::ClientDaemonJobKind::Devtool
            && job.label == label
            && !known_jobs.contains(&job.id)
    }) else {
        return DaemonDevtoolModifyCompletion::Pending;
    };
    match job.lifecycle {
        yoctui_model::ClientDaemonLifecycle::Exited => DaemonDevtoolModifyCompletion::Succeeded,
        yoctui_model::ClientDaemonLifecycle::Failed | yoctui_model::ClientDaemonLifecycle::Lost => {
            DaemonDevtoolModifyCompletion::Failed
        }
        _ => DaemonDevtoolModifyCompletion::Pending,
    }
}

pub(crate) fn local_workspace_effect_route(effect: &Effect) -> LocalWorkspaceEffectRoute {
    match effect {
        Effect::GetImageArtifacts(_) => LocalWorkspaceEffectRoute::ImageArtifacts,
        Effect::GetRootfsComposition(_) => LocalWorkspaceEffectRoute::RootfsComposition,
        _ => LocalWorkspaceEffectRoute::Other,
    }
}
