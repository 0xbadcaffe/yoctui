use super::*;

#[test]
fn daemon_client_batches_task_progress_without_losing_failure() {
    use yoctui_protocol::daemon::{DaemonBuildEvent, DaemonEvent, SequencedEvent};

    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([5; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut client = DaemonClientSnapshot::default();
    let mut app = yoctui_model::App::new(16, 4096);
    client.replace_app(&mut app, daemon_protocol_snapshot(&state));
    let task = |sequence, event| SequencedEvent {
        sequence,
        generation: sequence,
        event: DaemonEvent::Build(event),
    };
    let events = vec![
        task(
            1,
            DaemonBuildEvent::TaskStarted {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                pid: Some(42),
                worker: None,
                log_path: None,
                stats: None,
                started_unix_ms: None,
            },
        ),
        task(
            2,
            DaemonBuildEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(10),
            },
        ),
        task(
            3,
            DaemonBuildEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(90),
            },
        ),
        task(
            4,
            DaemonBuildEvent::TaskCompleted {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                success: false,
                started_unix_ms: None,
                finished_unix_ms: None,
            },
        ),
    ];

    client.apply_task_events_to_app(&mut app, &events).unwrap();
    assert_eq!(client.resume_cursor().unwrap().last_sequence, 4);
    assert_eq!(app.task_progress_events, 2);
    assert_eq!(app.task_progress_coalesced, 1);
    assert!(app.tasks.is_empty());
    assert_eq!(app.completed_tasks.len(), 1);
    assert!(!app.completed_tasks[0].success);
    assert_eq!(
        app.completed_tasks[0].task.state,
        yoctui_model::TaskState::Failed
    );
    assert!(matches!(
        client.snapshot.as_ref().unwrap().build_events.last(),
        Some(DaemonBuildEvent::TaskCompleted { success: false, .. })
    ));
}
