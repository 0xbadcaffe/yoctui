use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::QemuSessionRunning { id } => {
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
        }
        Action::AppendQemuSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        } => {
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs.append_output(
                job_id,
                BackgroundJobOutputEntry {
                    severity: if stream == QemuOutputStream::Stderr {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    message: line,
                    source: match stream {
                        QemuOutputStream::Stdout => BackgroundJobOutputSource::Stdout,
                        QemuOutputStream::Stderr => BackgroundJobOutputSource::Stderr,
                    },
                    truncated,
                    timestamp,
                },
            );
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
