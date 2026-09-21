use super::*;
use yoctui_protocol::daemon::{JobKind, JobSummary, LifecycleState};

fn snapshot(ids: &[u64]) -> DaemonSnapshot {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        1,
        "fixture-boot".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = yoctui_app::daemon_protocol_snapshot(&state);
    snapshot.jobs = ids
        .iter()
        .map(|id| JobSummary {
            id: JobId(*id),
            kind: JobKind::BitBakeBuild,
            label: format!("recovered-{id}"),
            lifecycle: LifecycleState::Failed,
            progress_current: None,
            progress_total: None,
            exit_code: Some(1),
        })
        .collect();
    snapshot
}

mod daemon_job_identity_recovery_preserves_terminal_rows_and_raw_namespace;

mod daemon_job_identity_exhaustion_never_wraps_or_enters_reserved_namespace;

mod daemon_job_identity_clones_allocate_unique_ids_concurrently;
