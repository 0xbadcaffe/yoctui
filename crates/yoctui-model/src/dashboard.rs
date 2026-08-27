//! Typed operational-dashboard projection over existing reducer authority.

use crate::*;
use std::{path::Path, time::SystemTime};

pub const DASHBOARD_COLLECTION_LIMIT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardNextActionKind {
    ReviewFailures,
    MonitorTasks,
    InspectArtifacts,
    ConfigureEnvironment,
    StartBuild,
}

impl DashboardNextActionKind {
    pub const fn action_id(self) -> &'static str {
        match self {
            Self::ReviewFailures => "dashboard.errors",
            Self::MonitorTasks => "dashboard.tasks",
            Self::InspectArtifacts => "dashboard.artifacts",
            Self::ConfigureEnvironment => "dashboard.environment",
            Self::StartBuild => "dashboard.build",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardNextAction {
    pub kind: DashboardNextActionKind,
    pub label: String,
    pub shortcut: String,
    pub state: WorkspaceAvailabilityState,
    pub enabled: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardArtifactSource {
    BackgroundJob,
    ImageInventory,
    SdkInventory,
}

impl DashboardArtifactSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::BackgroundJob => "job",
            Self::ImageInventory => "image",
            Self::SdkInventory => "SDK",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardArtifactRef<'a> {
    pub path: &'a Path,
    pub source: DashboardArtifactSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardEnvironmentState {
    Ready,
    NeedsConfiguration,
    Synchronizing,
    Disconnected,
}

impl DashboardEnvironmentState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NeedsConfiguration => "configuration required",
            Self::Synchronizing => "synchronizing",
            Self::Disconnected => "daemon disconnected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardHealthProjection {
    pub environment: DashboardEnvironmentState,
    pub replica: ClientReplicaStatus,
    pub bitbake: ClientDaemonLifecycle,
    pub build_filesystem_sample: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardProjection<'a> {
    pub summary: BuildSummary,
    pub progress: ProgressHierarchy,
    pub next_action: DashboardNextAction,
    pub failures: Vec<&'a LogEntry>,
    pub recent_work: Vec<JobHistoryRowRef<'a>>,
    pub artifacts: Vec<DashboardArtifactRef<'a>>,
    pub health: DashboardHealthProjection,
}

fn push_dashboard_artifact<'a>(
    artifacts: &mut Vec<DashboardArtifactRef<'a>>,
    path: &'a Path,
    source: DashboardArtifactSource,
) {
    if artifacts.len() < DASHBOARD_COLLECTION_LIMIT
        && !artifacts.iter().any(|artifact| artifact.path == path)
    {
        artifacts.push(DashboardArtifactRef { path, source });
    }
}

impl App {
    pub fn dashboard_projection_at(&self, now: SystemTime) -> DashboardProjection<'_> {
        let failures = self
            .logs
            .entries
            .iter()
            .rev()
            .filter(|entry| matches!(entry.severity, Severity::Warning | Severity::Error))
            .take(DASHBOARD_COLLECTION_LIMIT)
            .collect::<Vec<_>>();
        let recent_work = self
            .job_history_rows()
            .into_iter()
            .take(DASHBOARD_COLLECTION_LIMIT)
            .collect::<Vec<_>>();
        let artifacts = self.dashboard_artifacts();
        let environment_ready = self.build_environment.connected()
            || (self.workspace.source_dir.is_some() && self.workspace.build_dir.is_some());
        let environment = match self.daemon.status {
            ClientReplicaStatus::Disconnected => DashboardEnvironmentState::Disconnected,
            ClientReplicaStatus::Synchronizing | ClientReplicaStatus::Stale => {
                DashboardEnvironmentState::Synchronizing
            }
            ClientReplicaStatus::Current if environment_ready => DashboardEnvironmentState::Ready,
            ClientReplicaStatus::Current => DashboardEnvironmentState::NeedsConfiguration,
        };
        let next_kind = if self.build.errors > 0
            || self.build.status == BuildStatus::Failed
            || failures
                .iter()
                .any(|entry| entry.severity == Severity::Error)
        {
            DashboardNextActionKind::ReviewFailures
        } else if matches!(
            self.build.status,
            BuildStatus::LoadingWorkspace
                | BuildStatus::Parsing
                | BuildStatus::Running
                | BuildStatus::Cancelling
        ) {
            DashboardNextActionKind::MonitorTasks
        } else if self.build.status == BuildStatus::Completed && !artifacts.is_empty() {
            DashboardNextActionKind::InspectArtifacts
        } else if !environment_ready {
            DashboardNextActionKind::ConfigureEnvironment
        } else {
            DashboardNextActionKind::StartBuild
        };
        let next_action = compatibility_ui_workspace_action_presentations(
            &self.workspace_compatibility,
            WorkspaceDestination::Dashboard,
        )
        .into_iter()
        .find(|action| action.id == next_kind.action_id())
        .map(|action| DashboardNextAction {
            kind: next_kind,
            label: action.label.into(),
            shortcut: action.shortcut.into(),
            state: action.availability.state,
            enabled: action.availability.enabled,
            reason: action.availability.exact_reason(),
        })
        .unwrap_or_else(|| DashboardNextAction {
            kind: next_kind,
            label: "Dashboard action unavailable".into(),
            shortcut: "–".into(),
            state: WorkspaceAvailabilityState::Unknown,
            enabled: false,
            reason: Some("The typed dashboard action catalog is incomplete.".into()),
        });
        let progress = self.progress_hierarchy_at(now);
        DashboardProjection {
            summary: self.build_summary_at(now),
            progress,
            next_action,
            failures,
            recent_work,
            artifacts,
            health: DashboardHealthProjection {
                environment,
                replica: self.daemon.status,
                bitbake: self.daemon.bitbake,
                build_filesystem_sample: self.workspace.build_dir.is_some()
                    && self.host_telemetry.disk_total_bytes.is_some()
                    && self.host_telemetry.disk_available_bytes.is_some(),
            },
        }
    }

    fn dashboard_artifacts(&self) -> Vec<DashboardArtifactRef<'_>> {
        let mut artifacts = Vec::new();
        for job in self.background_jobs.jobs.iter().rev() {
            if let Some(result) = job.result.as_ref() {
                for path in result.artifacts.iter().rev() {
                    push_dashboard_artifact(
                        &mut artifacts,
                        path,
                        DashboardArtifactSource::BackgroundJob,
                    );
                }
            }
        }
        if let Some(inventory) = self.image_artifacts.artifacts() {
            for artifact in inventory.iter().rev() {
                push_dashboard_artifact(
                    &mut artifacts,
                    artifact.identity.path.as_path(),
                    DashboardArtifactSource::ImageInventory,
                );
            }
        }
        if let Some(inventory) = self.sdk_artifacts.artifacts() {
            for artifact in inventory.iter().rev() {
                push_dashboard_artifact(
                    &mut artifacts,
                    artifact.identity.path.as_path(),
                    DashboardArtifactSource::SdkInventory,
                );
            }
        }
        artifacts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn log(severity: Severity, index: u64) -> LogEntry {
        LogEntry {
            id: index,
            severity,
            message: format!("diagnostic-{index}"),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
            build: None,
            protected: false,
            diagnostic: None,
        }
    }

    #[test]
    fn ux_dashboard_prioritizes_failures_then_active_work_without_duplicating_state() {
        let mut app = App::new(16, 4_096);
        app.daemon.status = ClientReplicaStatus::Current;
        app.workspace.source_dir = Some("/work/poky".into());
        app.workspace.build_dir = Some("/work/poky/build".into());
        app.build.status = BuildStatus::Running;
        app.tasks.insert(
            TaskId("busybox:do_compile".into()),
            TaskInfo::active(
                TaskId("busybox:do_compile".into()),
                "busybox".into(),
                "do_compile".into(),
            ),
        );
        let running = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
        assert_eq!(
            running.next_action.kind,
            DashboardNextActionKind::MonitorTasks
        );
        assert_eq!(running.summary.active, 1);
        assert_eq!(running.health.environment, DashboardEnvironmentState::Ready);

        for index in 1..=6 {
            let _ = update(
                &mut app,
                Action::Log(log(
                    if index == 5 {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    index,
                )),
            );
        }
        let failed = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
        assert_eq!(
            failed.next_action.kind,
            DashboardNextActionKind::ReviewFailures
        );
        assert_eq!(failed.failures.len(), DASHBOARD_COLLECTION_LIMIT);
        assert_eq!(failed.failures[0].message, "diagnostic-6");
    }

    #[test]
    fn ux_dashboard_projects_environment_build_and_artifact_next_actions_honestly() {
        let mut app = App::new(16, 4_096);
        app.daemon.status = ClientReplicaStatus::Current;
        app.build_environment = BuildEnvironmentState::Unconfigured;
        let unconfigured = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
        assert_eq!(
            unconfigured.next_action.kind,
            DashboardNextActionKind::ConfigureEnvironment
        );
        assert_eq!(
            unconfigured.health.environment,
            DashboardEnvironmentState::NeedsConfiguration
        );

        app.workspace.source_dir = Some("/work/poky".into());
        app.workspace.build_dir = Some("/work/poky/build".into());
        let build = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
        assert_eq!(build.next_action.kind, DashboardNextActionKind::StartBuild);
        assert_eq!(build.next_action.state, WorkspaceAvailabilityState::Unknown);
        assert!(build.next_action.reason.is_some());

        let id = BackgroundJobId(7);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Build,
                title: "image build".into(),
                context: BackgroundJobContext::default(),
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id,
                started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: BackgroundJobResult {
                    summary: "built".into(),
                    artifacts: vec!["/deploy/core-image-minimal.wic".into()],
                },
                finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            },
        );
        app.build.status = BuildStatus::Completed;
        let completed = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
        assert_eq!(
            completed.next_action.kind,
            DashboardNextActionKind::InspectArtifacts
        );
        assert_eq!(completed.artifacts.len(), 1);
        assert_eq!(
            completed.artifacts[0].path,
            Path::new("/deploy/core-image-minimal.wic")
        );
        assert_eq!(completed.recent_work.len(), 1);
    }
}
