//! Typed operational-dashboard projection over existing reducer authority.

use crate::*;
use std::{path::Path, time::SystemTime};

pub const DASHBOARD_COLLECTION_LIMIT: usize = 4;
pub const COMMAND_CENTER_COLLECTION_LIMIT: usize = 3;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCenterFavoriteRef<'a> {
    pub favorite: &'a RawFavorite,
    pub projection: RawFavoriteProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCenterProjection<'a> {
    pub dashboard: DashboardProjection<'a>,
    pub recent_contexts: Vec<JobHistoryRowRef<'a>>,
    pub active_jobs: Vec<&'a BackgroundJob>,
    pub favorite_commands: Vec<CommandCenterFavoriteRef<'a>>,
    pub terminals: Vec<&'a ClientDaemonPtySummary>,
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
    pub fn command_center_projection_at(&self, now: SystemTime) -> CommandCenterProjection<'_> {
        let recent_contexts = self
            .job_history_rows()
            .into_iter()
            .filter(|row| match row {
                JobHistoryRowRef::Background(job) => job.context != BackgroundJobContext::default(),
                JobHistoryRowRef::Build(record) => record.target.is_some(),
            })
            .take(COMMAND_CENTER_COLLECTION_LIMIT)
            .collect();
        let active_jobs = self
            .background_jobs
            .jobs
            .iter()
            .rev()
            .filter(|job| !job.status.is_terminal())
            .take(COMMAND_CENTER_COLLECTION_LIMIT)
            .collect();
        let catalog = builtin_raw_catalog();
        let authority = self.workspace_compatibility.authority();
        let favorite_commands = self
            .raw_mode
            .favorites
            .iter()
            .take(COMMAND_CENTER_COLLECTION_LIMIT)
            .map(|favorite| CommandCenterFavoriteRef {
                favorite,
                projection: favorite.project(catalog, authority),
            })
            .collect();
        let selected_terminal = self.daemon.pty_sessions.get(self.pty_selection);
        let terminals =
            selected_terminal
                .into_iter()
                .chain(self.daemon.pty_sessions.iter().enumerate().filter_map(
                    |(index, terminal)| (index != self.pty_selection).then_some(terminal),
                ))
                .take(COMMAND_CENTER_COLLECTION_LIMIT)
                .collect();
        CommandCenterProjection {
            dashboard: self.dashboard_projection_at(now),
            recent_contexts,
            active_jobs,
            favorite_commands,
            terminals,
        }
    }

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

    #[test]
    fn ux_command_center_borrows_bounded_contexts_work_favorites_and_terminals() {
        let mut app = App::new(16, 4_096);
        for id in 1..=5 {
            let _ = update(
                &mut app,
                Action::QueueBackgroundJob(BackgroundJobSpec {
                    id: BackgroundJobId(id),
                    kind: BackgroundJobKind::Build,
                    title: format!("build-{id}"),
                    context: BackgroundJobContext {
                        workspace: Some(Screen::Recipes),
                        target: Some(format!("image-{id}")),
                        recipe: Some(format!("recipe-{id}")),
                        ..BackgroundJobContext::default()
                    },
                    cancellation_supported: true,
                    queued_at: SystemTime::UNIX_EPOCH + Duration::from_secs(id),
                }),
            );
            app.daemon.pty_sessions.push(ClientDaemonPtySummary {
                id,
                name: format!("shell-{id}"),
                lifecycle: ClientDaemonLifecycle::Running,
                viewers: 1,
            });
        }
        let command = builtin_raw_catalog()
            .commands
            .iter()
            .find(|command| {
                command.parameters.is_empty()
                    && matches!(command.execution, RawExecutionPolicy::Executable { .. })
            })
            .expect("the built-in catalog retains a parameterless executable command");
        app.raw_mode.favorites.push(
            RawFavorite::new(
                command,
                "Inspect environment",
                Default::default(),
                RawAdditionalArguments::from_vec(Vec::new()).unwrap(),
                0,
            )
            .unwrap(),
        );
        app.pty_selection = 4;

        let center = app.command_center_projection_at(SystemTime::UNIX_EPOCH);
        assert_eq!(
            center.recent_contexts.len(),
            COMMAND_CENTER_COLLECTION_LIMIT
        );
        assert_eq!(center.active_jobs.len(), COMMAND_CENTER_COLLECTION_LIMIT);
        assert_eq!(center.active_jobs[0].id, BackgroundJobId(5));
        assert_eq!(center.favorite_commands.len(), 1);
        assert_eq!(
            center.favorite_commands[0].favorite.name,
            "Inspect environment"
        );
        assert_eq!(center.terminals.len(), COMMAND_CENTER_COLLECTION_LIMIT);
        assert_eq!(
            center.terminals[0].id, 5,
            "selected terminal projects first"
        );
        assert_eq!(center.terminals[1].id, 1);

        let _ = update(&mut app, Action::OpenRawFavorites);
        assert_eq!(app.screen, Screen::RawMode);
        assert_eq!(app.raw_mode.view, RawModeView::Favorites);
    }
}
