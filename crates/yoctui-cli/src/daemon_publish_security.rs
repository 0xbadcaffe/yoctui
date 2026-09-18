//! Daemon publish security.
use super::*;

#[cfg(unix)]
pub(crate) fn publish_daemon_security_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_security::DaemonSecurityEvent,
) -> Result<()> {
    use daemon_security::DaemonSecurityEvent;
    use yoctui_protocol::daemon::{DaemonEvent, JobKind, JobSummary, LifecycleState};
    let (job_id, generation) = match &event {
        DaemonSecurityEvent::Started { job_id, generation }
        | DaemonSecurityEvent::Completed {
            job_id, generation, ..
        }
        | DaemonSecurityEvent::Failed {
            job_id, generation, ..
        }
        | DaemonSecurityEvent::Cancelled { job_id, generation } => (*job_id, *generation),
    };
    let label = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.label.clone())
        .unwrap_or_else(|| format!("Security report scan {generation}"));
    let mapped = match event {
        DaemonSecurityEvent::Started { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Security,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonSecurityEvent::Completed {
            reports,
            limitations,
            ..
        } => {
            journal.publish(DaemonEvent::SecuritySnapshot(
                yoctui_protocol::daemon::DaemonSecuritySnapshot {
                    generation,
                    reports,
                    limitations,
                },
            ))?;
            DaemonEvent::JobChanged(JobSummary {
                id: job_id,
                kind: JobKind::Security,
                label,
                lifecycle: LifecycleState::Exited,
                progress_current: None,
                progress_total: None,
                exit_code: Some(0),
            })
        }
        DaemonSecurityEvent::Failed { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Security,
            label: format!("{label}: {message}"),
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code: Some(1),
        }),
        DaemonSecurityEvent::Cancelled { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Security,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
    };
    journal.publish(mapped)?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn publish_daemon_security_mapper_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_security::DaemonSecurityMapperEvent,
) -> Result<()> {
    use daemon_security::DaemonSecurityMapperEvent;
    use yoctui_protocol::daemon::{
        DaemonEvent, JobKind, JobSummary, LifecycleState, LogRecord, LogSeverity,
    };
    let job_id = match &event {
        DaemonSecurityMapperEvent::Started { job_id, .. }
        | DaemonSecurityMapperEvent::Output { job_id, .. }
        | DaemonSecurityMapperEvent::Completed { job_id, .. }
        | DaemonSecurityMapperEvent::Failed { job_id, .. }
        | DaemonSecurityMapperEvent::Cancelled { job_id, .. }
        | DaemonSecurityMapperEvent::Lost { job_id, .. } => *job_id,
    };
    let label = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.label.clone())
        .unwrap_or_else(|| "Security package map".into());
    let mapped = match event {
        DaemonSecurityMapperEvent::Started { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Security,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonSecurityMapperEvent::Output {
            stream,
            line,
            truncated,
            ..
        } => DaemonEvent::Log(LogRecord {
            source: format!("security-map:{stream:?}"),
            severity: if matches!(stream, yoctui_model::SecurityOutputStream::Stderr) {
                LogSeverity::Warning
            } else {
                LogSeverity::Info
            },
            message: if truncated {
                format!("{line} [truncated]")
            } else {
                line
            },
            unix_ms: unix_ms(),
            recipe: None,
            task: None,
            path: None,
            build: None,
        }),
        DaemonSecurityMapperEvent::Completed { exit_code, .. } => {
            DaemonEvent::JobChanged(JobSummary {
                id: job_id,
                kind: JobKind::Security,
                label,
                lifecycle: LifecycleState::Exited,
                progress_current: None,
                progress_total: None,
                exit_code,
            })
        }
        DaemonSecurityMapperEvent::Failed { exit_code, .. } => {
            DaemonEvent::JobChanged(JobSummary {
                id: job_id,
                kind: JobKind::Security,
                label,
                lifecycle: LifecycleState::Failed,
                progress_current: None,
                progress_total: None,
                exit_code,
            })
        }
        DaemonSecurityMapperEvent::Cancelled { exit_code, .. } => {
            DaemonEvent::JobChanged(JobSummary {
                id: job_id,
                kind: JobKind::Security,
                label,
                lifecycle: LifecycleState::Exited,
                progress_current: None,
                progress_total: None,
                exit_code,
            })
        }
        DaemonSecurityMapperEvent::Lost { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Security,
            label: format!("{label}: {message}"),
            lifecycle: LifecycleState::Lost,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
    };
    journal.publish(mapped)?;
    Ok(())
}
