pub(crate) fn model_task_event_from_daemon(
    event: &yoctui_protocol::daemon::DaemonBuildEvent,
) -> Option<yoctui_model::TaskEvent> {
    match model_action_from_backend_event(backend_event_from_daemon(event.clone())?)? {
        yoctui_model::Action::TaskStarted(mut task) => {
            let yoctui_protocol::daemon::DaemonBuildEvent::TaskStarted {
                started_unix_ms, ..
            } = event
            else {
                return None;
            };
            task.started = observed_daemon_time(*started_unix_ms);
            Some(yoctui_model::TaskEvent::ObservedStarted(task))
        }
        yoctui_model::Action::TaskQueued(task) => Some(yoctui_model::TaskEvent::Queued(task)),
        yoctui_model::Action::TaskProgress { id, progress } => {
            Some(yoctui_model::TaskEvent::Progress { id, progress })
        }
        yoctui_model::Action::TaskCompleted { id, success } => {
            let yoctui_protocol::daemon::DaemonBuildEvent::TaskCompleted {
                started_unix_ms,
                finished_unix_ms,
                ..
            } = event
            else {
                return None;
            };
            Some(yoctui_model::TaskEvent::ObservedCompleted {
                id,
                success,
                timing: yoctui_model::ObservedTaskTiming {
                    started: observed_daemon_time(*started_unix_ms),
                    finished: observed_daemon_time(*finished_unix_ms),
                },
            })
        }
        _ => None,
    }
}

pub(crate) fn backend_event_from_daemon(
    event: yoctui_protocol::daemon::DaemonBuildEvent,
) -> Option<BackendEvent> {
    use yoctui_protocol::daemon::DaemonBuildEvent;
    Some(match event {
        DaemonBuildEvent::Reset { .. } => return None,
        DaemonBuildEvent::Workspace { data } => BackendEvent::Workspace(yoctui_model::Workspace {
            build_dir: data.build_dir.map(Into::into),
            source_dir: data.source_dir.map(Into::into),
            variables: data.variables,
            variable_provenance: data.variable_provenance,
            variable_provenance_chain: data.variable_provenance_chain,
            bitbake_version: data.bitbake_version,
            release: data.release,
            layers: data
                .layers
                .into_iter()
                .map(|layer| yoctui_model::Layer {
                    name: layer.name,
                    path: layer.path.into(),
                    priority: layer.priority,
                })
                .collect(),
            recipes: data
                .recipes
                .into_iter()
                .map(|recipe| yoctui_model::Recipe {
                    name: recipe.name,
                    version: recipe.version,
                    layer: recipe.layer,
                    preferred_version: recipe.preferred_version,
                    file: recipe.file.map(Into::into),
                    append_count: recipe.append_count,
                })
                .collect(),
        }),
        DaemonBuildEvent::Started { .. } => BackendEvent::BuildStarted,
        DaemonBuildEvent::ParseProgress { current, total } => {
            BackendEvent::ParseProgress { current, total }
        }
        DaemonBuildEvent::SstateSummary { summary } => BackendEvent::SstateSummary(summary),
        DaemonBuildEvent::TaskQueued {
            recipe,
            task,
            worker,
            stats,
        } => BackendEvent::TaskQueued {
            recipe,
            task,
            worker,
            stats: stats.map(daemon_task_stats),
        },
        DaemonBuildEvent::TaskStarted {
            recipe,
            task,
            pid,
            worker,
            log_path,
            stats,
            ..
        } => BackendEvent::TaskStarted {
            recipe,
            task,
            pid,
            worker,
            log_path: log_path.map(Into::into),
            stats: stats.map(daemon_task_stats),
        },
        DaemonBuildEvent::TaskProgress {
            recipe,
            task,
            progress,
        } => BackendEvent::TaskProgress {
            recipe,
            task,
            progress,
        },
        DaemonBuildEvent::TaskCompleted {
            recipe,
            task,
            success,
            ..
        } => BackendEvent::TaskCompleted {
            recipe,
            task,
            success,
        },
        DaemonBuildEvent::Completed {
            success, exit_code, ..
        } => BackendEvent::BuildCompleted { success, exit_code },
        DaemonBuildEvent::CommandFailed { code, message } => {
            BackendEvent::CommandFailed { code, message }
        }
        DaemonBuildEvent::Disconnected => BackendEvent::Disconnected,
    })
}

pub(crate) fn daemon_task_stats(stats: yoctui_protocol::TaskStatsData) -> yoctui_model::TaskStats {
    yoctui_model::TaskStats {
        completed: stats.completed,
        total: stats.total,
        active: stats.active,
        failed: stats.failed,
    }
}

pub(crate) fn install_daemon_build_environment(app: &mut yoctui_model::App) {
    let (Some(source_dir), Some(build_dir)) = (
        app.workspace.source_dir.clone(),
        app.workspace.build_dir.clone(),
    ) else {
        return;
    };
    app.build_environment =
        yoctui_model::BuildEnvironmentState::Connected(yoctui_model::BuildEnvironmentProfile {
            init_script: source_dir.join("oe-init-build-env"),
            source_dir,
            build_dir,
        });
}
