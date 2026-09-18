//! Daemon publish test.
use super::*;

#[cfg(unix)]
pub(crate) fn publish_daemon_test_event(
    journal: &mut yoctui_protocol::daemon::DaemonSnapshotJournal,
    event: daemon_test::DaemonTestEvent,
) -> Result<()> {
    use daemon_test::DaemonTestEvent;
    use yoctui_protocol::daemon::{
        DaemonEvent, JobKind, JobSummary, LifecycleState, LogRecord, LogSeverity,
    };
    if let DaemonTestEvent::Snapshot { snapshot, .. } = &event {
        journal.publish(DaemonEvent::TestResults(snapshot.clone()))?;
    }
    let job_id = match &event {
        DaemonTestEvent::Started { job_id, .. }
        | DaemonTestEvent::Output { job_id, .. }
        | DaemonTestEvent::Completed { job_id, .. }
        | DaemonTestEvent::Failed { job_id, .. }
        | DaemonTestEvent::Cancelled { job_id, .. }
        | DaemonTestEvent::TimedOut { job_id, .. }
        | DaemonTestEvent::Lost { job_id, .. }
        | DaemonTestEvent::Snapshot { job_id, .. } => *job_id,
        DaemonTestEvent::Comparison { job_id, .. } => *job_id,
        DaemonTestEvent::JunitCompleted { job_id, .. } => *job_id,
    };
    let label = journal
        .snapshot()
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.label.clone())
        .unwrap_or_else(|| "Test".into());
    let mapped = match event {
        DaemonTestEvent::JunitCompleted {
            success, message, ..
        } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Testing,
            label: message.unwrap_or_else(|| "JUnit export".into()),
            lifecycle: if success {
                LifecycleState::Exited
            } else {
                LifecycleState::Failed
            },
            progress_current: None,
            progress_total: None,
            exit_code: Some(if success { 0 } else { 1 }),
        }),
        DaemonTestEvent::Comparison { diff, .. } => DaemonEvent::TestComparison(diff),
        DaemonTestEvent::Snapshot { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Testing,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code: Some(0),
        }),
        DaemonTestEvent::Started { .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Testing,
            label,
            lifecycle: LifecycleState::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        }),
        DaemonTestEvent::Output {
            stream,
            line,
            truncated,
            ..
        } => DaemonEvent::Log(LogRecord {
            source: format!("test:{stream:?}"),
            severity: if matches!(stream, yoctui_model::TestOutputStream::Stderr) {
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
        DaemonTestEvent::Completed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Testing,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonTestEvent::Failed { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Testing,
            label,
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonTestEvent::Cancelled { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Testing,
            label,
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonTestEvent::TimedOut { exit_code, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Testing,
            label,
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code,
        }),
        DaemonTestEvent::Lost { message, .. } => DaemonEvent::JobChanged(JobSummary {
            id: job_id,
            kind: JobKind::Testing,
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
