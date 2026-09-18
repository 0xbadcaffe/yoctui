use super::*;

#[test]
fn daemon_attach_build_maps_task_stats_into_protocol_and_job_progress() {
    let (event, job) = daemon_build_event(
        yoctui_bitbake::BackendEvent::TaskStarted {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: Some("worker-1".into()),
            log_path: None,
            stats: Some(yoctui_model::TaskStats {
                completed: 102,
                total: 4090,
                active: 8,
                failed: 0,
            }),
        },
        yoctui_protocol::daemon::JobId(7),
    );
    assert!(matches!(
        event,
        Some(yoctui_protocol::daemon::DaemonBuildEvent::TaskStarted {
            stats: Some(yoctui_protocol::TaskStatsData {
                completed: 102,
                total: 4090,
                ..
            }),
            ..
        })
    ));
    let job = job.unwrap();
    assert_eq!(job.progress_current, Some(102));
    assert_eq!(job.progress_total, Some(4090));
}
