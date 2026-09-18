use super::*;

#[test]
fn task_identity_unknown_statistics_use_existing_job_progress_without_a_task_row() {
    let (event, job) = daemon_build_event(
        yoctui_bitbake::BackendEvent::TaskStats(yoctui_model::TaskStats {
            completed: 2340,
            total: 6812,
            active: 1,
            failed: 0,
        }),
        yoctui_protocol::daemon::JobId(42),
    );
    assert_eq!(event, None);
    let job = job.unwrap();
    assert_eq!(job.id.0, 42);
    assert_eq!(job.progress_current, Some(2340));
    assert_eq!(job.progress_total, Some(6812));
}
