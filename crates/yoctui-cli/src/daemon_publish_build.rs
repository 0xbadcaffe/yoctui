//! Daemon publish build.
use super::*;

#[cfg(unix)]
pub(crate) fn publish_startup_metadata_log(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    message: &str,
    failed: bool,
) -> Result<()> {
    use yoctui_protocol::daemon::{DaemonEvent, LogRecord, LogSeverity};
    journal.publish(DaemonEvent::Log(LogRecord {
        source: "daemon-metadata".into(),
        severity: if failed {
            LogSeverity::Error
        } else {
            LogSeverity::Info
        },
        message: message.into(),
        unix_ms: unix_ms(),
        recipe: None,
        task: None,
        path: None,
        build: None,
    }))?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn publish_daemon_bitbake_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_bitbake::DaemonBitBakeEvent,
) -> Result<()> {
    use daemon_bitbake::DaemonBitBakeEvent;
    use yoctui_bitbake::BackendEvent;
    use yoctui_protocol::daemon::{
        DaemonBuildEvent, DaemonEvent, JobKind, JobSummary, LifecycleState, LogRecord, LogSeverity,
    };
    match event {
        DaemonBitBakeEvent::Backend { job_id, event } => match *event {
            BackendEvent::Log(entry) => {
                journal.publish(DaemonEvent::Log(LogRecord {
                    source: "bitbake".into(),
                    severity: match entry.severity {
                        yoctui_model::Severity::Trace => LogSeverity::Trace,
                        yoctui_model::Severity::Info => LogSeverity::Info,
                        yoctui_model::Severity::Warning => LogSeverity::Warning,
                        yoctui_model::Severity::Error => LogSeverity::Error,
                    },
                    message: entry.message,
                    unix_ms: unix_ms(),
                    recipe: entry.recipe,
                    task: entry.task,
                    path: entry.path.map(|path| path.display().to_string()),
                    build: entry.build,
                }))?;
            }
            event => {
                let lifecycle = match &event {
                    BackendEvent::BuildStarted => Some(LifecycleState::Running),
                    BackendEvent::BuildCompleted { .. }
                    | BackendEvent::CommandFailed { .. }
                    | BackendEvent::Disconnected => Some(LifecycleState::Disconnected),
                    _ => None,
                };
                let (mapped, job_update) = daemon_build_event(event, job_id);
                if let Some(lifecycle) = lifecycle {
                    let mut bitbake = journal.snapshot().bitbake.clone();
                    bitbake.lifecycle = lifecycle;
                    bitbake.diagnostic = None;
                    journal.publish(DaemonEvent::BitBakeChanged(bitbake))?;
                }
                if let Some(mapped) = mapped {
                    journal.publish(DaemonEvent::Build(mapped))?;
                }
                if let Some(mut job) = job_update {
                    let existing_label = journal
                        .snapshot()
                        .jobs
                        .iter()
                        .find(|candidate| candidate.id == job.id)
                        .map(|candidate| candidate.label.clone());
                    preserve_bitbake_target_label(existing_label.as_deref(), &mut job);
                    journal.publish(DaemonEvent::JobChanged(job))?;
                }
            }
        },
        DaemonBitBakeEvent::Failed { job_id, message } => {
            journal.publish(DaemonEvent::Build(DaemonBuildEvent::CommandFailed {
                code: "daemon_backend".into(),
                message: message.clone(),
            }))?;
            journal.publish(DaemonEvent::JobChanged(JobSummary {
                id: job_id,
                kind: JobKind::BitBakeBuild,
                label: format!("BitBake build: {message}"),
                lifecycle: LifecycleState::Failed,
                progress_current: None,
                progress_total: None,
                exit_code: None,
            }))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn preserve_bitbake_target_label(
    existing_label: Option<&str>,
    job: &mut yoctui_protocol::daemon::JobSummary,
) {
    if let Some(existing_label) = existing_label
        && existing_label.starts_with("BitBake build ")
    {
        job.label = existing_label.to_owned();
    }
}

#[cfg(unix)]
pub(crate) fn daemon_build_event(
    event: yoctui_bitbake::BackendEvent,
    job_id: yoctui_protocol::daemon::JobId,
) -> (
    Option<yoctui_protocol::daemon::DaemonBuildEvent>,
    Option<yoctui_protocol::daemon::JobSummary>,
) {
    daemon_build_event_at(event, job_id, unix_ms())
}

#[cfg(unix)]
pub(crate) fn daemon_build_event_at(
    event: yoctui_bitbake::BackendEvent,
    job_id: yoctui_protocol::daemon::JobId,
    observed_unix_ms: u64,
) -> (
    Option<yoctui_protocol::daemon::DaemonBuildEvent>,
    Option<yoctui_protocol::daemon::JobSummary>,
) {
    use yoctui_bitbake::BackendEvent;
    use yoctui_protocol::daemon::{DaemonBuildEvent, JobKind, JobSummary, LifecycleState};
    use yoctui_protocol::{LayerData, RecipeData, TaskStatsData, WorkspaceData};
    let stats = |stats: yoctui_model::TaskStats| TaskStatsData {
        completed: stats.completed,
        total: stats.total,
        active: stats.active,
        failed: stats.failed,
    };
    let running_job = |progress: yoctui_model::TaskStats| JobSummary {
        id: job_id,
        kind: JobKind::BitBakeBuild,
        label: "BitBake build".into(),
        lifecycle: LifecycleState::Running,
        progress_current: Some(progress.completed as u64),
        progress_total: Some(progress.total as u64),
        exit_code: None,
    };
    match event {
        BackendEvent::Workspace(workspace) => (
            Some(DaemonBuildEvent::Workspace {
                data: WorkspaceData {
                    build_dir: workspace.build_dir.map(|path| path.display().to_string()),
                    source_dir: workspace.source_dir.map(|path| path.display().to_string()),
                    variables: workspace.variables,
                    variable_provenance: workspace.variable_provenance,
                    variable_provenance_chain: workspace.variable_provenance_chain,
                    bitbake_version: workspace.bitbake_version,
                    release: workspace.release,
                    layers: workspace
                        .layers
                        .into_iter()
                        .map(|layer| LayerData {
                            name: layer.name,
                            path: layer.path.display().to_string(),
                            priority: layer.priority,
                        })
                        .collect(),
                    recipes: workspace
                        .recipes
                        .into_iter()
                        .map(|recipe| RecipeData {
                            name: recipe.name,
                            version: recipe.version,
                            layer: recipe.layer,
                            preferred_version: recipe.preferred_version,
                            file: recipe.file.map(|path| path.display().to_string()),
                            append_count: recipe.append_count,
                        })
                        .collect(),
                },
            }),
            None,
        ),
        BackendEvent::BuildStarted => (
            Some(DaemonBuildEvent::Started {
                started_unix_ms: Some(observed_unix_ms),
            }),
            Some(JobSummary {
                id: job_id,
                kind: JobKind::BitBakeBuild,
                label: "BitBake build".into(),
                lifecycle: LifecycleState::Running,
                progress_current: None,
                progress_total: None,
                exit_code: None,
            }),
        ),
        BackendEvent::ParseProgress { current, total } => (
            Some(DaemonBuildEvent::ParseProgress { current, total }),
            None,
        ),
        BackendEvent::SstateSummary(summary) => {
            (Some(DaemonBuildEvent::SstateSummary { summary }), None)
        }
        // An unresolved recipe contributes aggregate authority, not a fake
        // per-task row. Existing job progress keeps older clients decodable.
        BackendEvent::TaskStats(stats) => (None, Some(running_job(stats))),
        BackendEvent::TaskQueued {
            recipe,
            task,
            worker,
            stats: task_stats,
        } => {
            let job = task_stats.map(running_job);
            (
                Some(DaemonBuildEvent::TaskQueued {
                    recipe,
                    task,
                    worker,
                    stats: task_stats.map(stats),
                }),
                job,
            )
        }
        BackendEvent::TaskStarted {
            recipe,
            task,
            pid,
            worker,
            log_path,
            stats: task_stats,
        } => {
            let job = task_stats.map(running_job);
            (
                Some(DaemonBuildEvent::TaskStarted {
                    recipe,
                    task,
                    pid,
                    worker,
                    log_path: log_path.map(|path| path.display().to_string()),
                    stats: task_stats.map(stats),
                    started_unix_ms: Some(observed_unix_ms),
                }),
                job,
            )
        }
        BackendEvent::TaskProgress {
            recipe,
            task,
            progress,
        } => (
            Some(DaemonBuildEvent::TaskProgress {
                recipe,
                task,
                progress,
            }),
            None,
        ),
        BackendEvent::TaskCompleted {
            recipe,
            task,
            success,
        } => (
            Some(DaemonBuildEvent::TaskCompleted {
                recipe,
                task,
                success,
                started_unix_ms: None,
                finished_unix_ms: Some(observed_unix_ms),
            }),
            None,
        ),
        BackendEvent::BuildCompleted { success, exit_code } => (
            Some(DaemonBuildEvent::Completed {
                success,
                exit_code,
                finished_unix_ms: Some(observed_unix_ms),
            }),
            Some(JobSummary {
                id: job_id,
                kind: JobKind::BitBakeBuild,
                label: "BitBake build".into(),
                lifecycle: if success {
                    LifecycleState::Exited
                } else {
                    LifecycleState::Failed
                },
                progress_current: None,
                progress_total: None,
                exit_code,
            }),
        ),
        BackendEvent::CommandFailed { code, message } => (
            Some(DaemonBuildEvent::CommandFailed { code, message }),
            Some(JobSummary {
                id: job_id,
                kind: JobKind::BitBakeBuild,
                label: "BitBake build failed".into(),
                lifecycle: LifecycleState::Failed,
                progress_current: None,
                progress_total: None,
                exit_code: None,
            }),
        ),
        BackendEvent::Disconnected => (
            Some(DaemonBuildEvent::Disconnected),
            Some(JobSummary {
                id: job_id,
                kind: JobKind::BitBakeBuild,
                label: "BitBake backend disconnected".into(),
                lifecycle: LifecycleState::Lost,
                progress_current: None,
                progress_total: None,
                exit_code: None,
            }),
        ),
        _ => (None, None),
    }
}
