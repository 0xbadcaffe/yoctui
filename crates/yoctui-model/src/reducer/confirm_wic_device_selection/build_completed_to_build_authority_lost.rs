use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::BuildCompleted { success, exit_code } => {
            archive_unfinished_tasks(app, TaskState::Lost, Some("build ended"));
            app.build.status = if success {
                BuildStatus::Completed
            } else {
                BuildStatus::Failed
            };
            if !success {
                app.build.errors = app.build.errors.max(1);
            }
            if success && let Some(total) = app.build.total {
                app.build.completed = total;
            }
            insert_system_log(
                app,
                if success {
                    Severity::Info
                } else {
                    Severity::Error
                },
                format!(
                    "Build {} with exit code {}",
                    if success { "completed" } else { "failed" },
                    exit_code.map_or_else(|| "unknown".into(), |code| code.to_string())
                ),
            );
            app.build.exit_code = exit_code;
            app.build_history.push_back(BuildRecord {
                target: app.build.target.clone(),
                success,
                exit_code,
                elapsed: app.elapsed(),
                completed_tasks: app.build.completed,
                warnings: app.build.warnings,
                errors: app.build.errors,
            });
            if app.build_history.len() > MAX_BUILD_HISTORY {
                app.build_history.pop_front();
            }
            app.build_history_selection = 0;
            clamp_task_selection(app);
            enqueue_build_completion(app);
            app.notification = Some(if success {
                if app.build.warnings > 0 {
                    format!(
                        "Build completed with {} warning(s). Open Errors to investigate.",
                        app.build.warnings
                    )
                } else {
                    "Build completed successfully with no errors.".into()
                }
            } else {
                format!(
                    "Build failed with {} error(s). Press Enter to open Errors.",
                    app.build.errors
                )
            });
        }
        Action::BuildAuthorityLost { message } => {
            if matches!(
                app.build.status,
                BuildStatus::LoadingWorkspace
                    | BuildStatus::Parsing
                    | BuildStatus::Running
                    | BuildStatus::Cancelling
            ) {
                archive_unfinished_tasks(app, TaskState::Lost, Some("build authority lost"));
                app.build.status = BuildStatus::Lost;
                app.build.exit_code = None;
                insert_system_log(
                    app,
                    Severity::Warning,
                    format!("Build authority lost: {message}"),
                );
                app.notification = Some(format!(
                    "Build authority lost: {message}. Reconnecting to the daemon."
                ));
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
