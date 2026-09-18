//! Daemon publish qa.
use super::*;

#[cfg(unix)]
pub(crate) fn publish_daemon_qa_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_qa::DaemonQaEvent,
) -> Result<()> {
    use daemon_qa::DaemonQaEvent;
    use yoctui_protocol::daemon::{
        DaemonEvent, JobKind, JobSummary, LifecycleState, LogRecord, LogSeverity,
    };
    let job_id = match &event {
        DaemonQaEvent::Started { job_id, .. }
        | DaemonQaEvent::Output { job_id, .. }
        | DaemonQaEvent::Completed { job_id, .. }
        | DaemonQaEvent::Failed { job_id, .. }
        | DaemonQaEvent::Cancelled { job_id, .. }
        | DaemonQaEvent::Lost { job_id, .. } => *job_id,
    };
    let label = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.label.clone())
        .unwrap_or_else(|| "QA layer check".into());
    let mapped = match event {
        DaemonQaEvent::Started { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qa,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonQaEvent::Output {
            stream,
            line,
            truncated,
            ..
        } => DaemonEvent::Log(LogRecord {
            source: format!("qa:{stream:?}"),
            severity: if matches!(stream, yoctui_model::QaOutputStream::Stderr) {
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
        DaemonQaEvent::Completed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qa,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonQaEvent::Failed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qa,
            label,
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonQaEvent::Cancelled { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qa,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonQaEvent::Lost { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qa,
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

#[cfg(unix)]
pub(crate) fn publish_daemon_qa_report_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_qa::DaemonQaReportEvent,
) -> Result<()> {
    use daemon_qa::DaemonQaReportEvent;
    use yoctui_protocol::daemon::{DaemonEvent, JobKind, JobSummary, LifecycleState};
    let (job_id, generation) = match &event {
        DaemonQaReportEvent::Started { job_id, generation }
        | DaemonQaReportEvent::Completed {
            job_id, generation, ..
        }
        | DaemonQaReportEvent::Failed {
            job_id, generation, ..
        }
        | DaemonQaReportEvent::Cancelled { job_id, generation } => (*job_id, *generation),
    };
    let label = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.label.clone())
        .unwrap_or_else(|| format!("QA report scan {generation}"));
    let mapped = match event {
        DaemonQaReportEvent::Started { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qa,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonQaReportEvent::Completed {
            reports,
            limitations,
            ..
        } => {
            journal.publish(DaemonEvent::QaSnapshot(
                yoctui_protocol::daemon::DaemonQaSnapshot {
                    generation,
                    capability: "reports".into(),
                    task_bindings: Vec::new(),
                    reports,
                    limitations,
                },
            ))?;
            DaemonEvent::JobChanged(JobSummary {
                id: job_id,
                kind: JobKind::Qa,
                label,
                lifecycle: LifecycleState::Exited,
                progress_current: None,
                progress_total: None,
                exit_code: Some(0),
            })
        }
        DaemonQaReportEvent::Failed { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qa,
            label: format!("{label}: {message}"),
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code: Some(1),
        }),
        DaemonQaReportEvent::Cancelled { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Qa,
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
