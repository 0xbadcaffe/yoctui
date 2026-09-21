use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::AppendTestSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        } => {
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs.append_output(
                job_id,
                BackgroundJobOutputEntry {
                    severity: if stream == TestOutputStream::Stderr {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    message: line,
                    source: if stream == TestOutputStream::Stderr {
                        BackgroundJobOutputSource::Stderr
                    } else {
                        BackgroundJobOutputSource::Stdout
                    },
                    truncated,
                    timestamp,
                },
            );
        }
        Action::CompleteTestSession {
            id,
            exit_code,
            result_paths,
            finished_at,
        } => {
            if !test_result_paths_are_valid(&result_paths) {
                note_stale_test_event(app);
                app.notification = Some("Testing returned invalid structured result paths.".into());
                return None;
            }
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.exit_code = Some(exit_code);
                session.result_paths.clone_from(&result_paths);
                session.outcome = Some(TestSessionOutcome::Succeeded);
            }) else {
                note_stale_test_event(app);
                return None;
            };
            let import_roots = result_paths.clone();
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Succeeded;
                    job.finished_at = Some(finished_at);
                    job.result = Some(BackgroundJobResult {
                        summary: "Testing operation completed".into(),
                        artifacts: result_paths,
                    });
                },
            );
            if !import_roots.is_empty()
                && app
                    .background_jobs
                    .get(job_id)
                    .is_some_and(|job| job.status == BackgroundJobStatus::Succeeded)
            {
                return begin_test_result_import(app, import_roots);
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
