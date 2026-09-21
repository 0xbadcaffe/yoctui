use super::*;

#[test]
fn live_tasks_preserves_queue_statistics_and_task_details() {
    let queued = BridgeBackend::event(Event::TaskQueued {
        recipe: "busybox".into(),
        task: "do_compile".into(),
        worker: Some("worker-1".into()),
        stats: Some(TaskStatsData {
            completed: 3,
            total: 10,
            active: 2,
            failed: 1,
        }),
    })
    .unwrap();
    assert!(matches!(
        queued,
        BackendEvent::TaskQueued {
            stats: Some(TaskStats { total: 10, .. }),
            ..
        }
    ));
    let started = BridgeBackend::event(Event::TaskStarted {
        recipe: "busybox".into(),
        task: "do_compile".into(),
        pid: Some(42),
        worker: Some("worker-1".into()),
        log_path: Some("/tmp/log.do_compile".into()),
        stats: None,
    })
    .unwrap();
    assert!(matches!(
        started,
        BackendEvent::TaskStarted {
            pid: Some(42),
            log_path: Some(path),
            ..
        } if path.as_os_str() == "/tmp/log.do_compile"
    ));
}
