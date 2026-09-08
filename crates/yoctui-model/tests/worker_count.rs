use yoctui_model::{
    Action, App, BuildStatus, ClientReplicaStatus, TaskId, TaskInfo, TaskState, update,
};

fn running_app() -> App {
    let mut app = App::new(64, 64 * 1024);
    app.daemon.status = ClientReplicaStatus::Current;
    let _ = update(&mut app, Action::BuildStarted);
    app
}

fn task(name: &str, pid: Option<u32>, worker: Option<&str>) -> TaskInfo {
    let mut task = TaskInfo::active(TaskId(name.into()), name.into(), "do_compile".into());
    task.pid = pid;
    task.worker = worker.map(str::to_owned);
    task
}

fn insert(app: &mut App, task: TaskInfo) {
    app.tasks.insert(task.id.clone(), task);
}

#[test]
fn worker_count_deduplicates_active_pids_and_ignores_other_states() {
    let mut app = running_app();
    insert(&mut app, task("one", Some(41), Some("label")));
    insert(&mut app, task("same-worker", Some(41), Some("label")));
    insert(&mut app, task("two", Some(42), Some("label")));
    for (index, state) in [
        TaskState::Queued,
        TaskState::Waiting,
        TaskState::Completed,
        TaskState::Failed,
        TaskState::Cancelled,
        TaskState::Lost,
    ]
    .into_iter()
    .enumerate()
    {
        let mut inactive = task(&format!("inactive-{index}"), Some(900), Some("other"));
        inactive.state = state;
        insert(&mut app, inactive);
    }
    assert_eq!(app.active_worker_count(), Some(2));
}

#[test]
fn worker_count_uses_complete_labels_only_when_pids_are_incomplete() {
    let mut app = running_app();
    insert(&mut app, task("one", None, Some("worker-a")));
    insert(&mut app, task("duplicate", Some(42), Some("worker-a")));
    insert(&mut app, task("two", Some(0), Some("worker-b")));
    assert_eq!(app.active_worker_count(), Some(2));
}

#[test]
fn worker_count_partial_mixed_and_empty_identity_never_becomes_zero() {
    let mut app = running_app();
    assert_eq!(app.active_worker_count(), None);
    insert(&mut app, task("pid-only", Some(41), None));
    insert(&mut app, task("label-only", None, Some("worker-b")));
    assert_eq!(app.active_worker_count(), None);
    for label in [None, Some(""), Some("  ")] {
        app.tasks.clear();
        insert(&mut app, task("unknown", Some(0), label));
        assert_eq!(app.active_worker_count(), None);
    }
}

#[test]
fn worker_count_requires_current_replica_and_known_build_lifecycle() {
    let mut app = running_app();
    insert(&mut app, task("one", Some(41), None));
    for replica in [
        ClientReplicaStatus::Disconnected,
        ClientReplicaStatus::Synchronizing,
        ClientReplicaStatus::Stale,
    ] {
        app.daemon.status = replica;
        assert_eq!(app.active_worker_count(), None);
    }
    app.daemon.status = ClientReplicaStatus::Current;
    for status in [
        BuildStatus::Lost,
        BuildStatus::LoadingWorkspace,
        BuildStatus::Parsing,
    ] {
        app.build.status = status;
        assert_eq!(app.active_worker_count(), None);
    }
    app.build.status = BuildStatus::Cancelling;
    assert_eq!(app.active_worker_count(), Some(1));
}

#[test]
fn worker_count_terminal_completion_and_next_build_reset_do_not_reuse_workers() {
    let mut app = running_app();
    let _ = update(&mut app, Action::TaskStarted(task("one", Some(41), None)));
    assert_eq!(app.active_worker_count(), Some(1));
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert_eq!(app.active_worker_count(), Some(0));
    for status in [
        BuildStatus::Idle,
        BuildStatus::Failed,
        BuildStatus::Cancelled,
    ] {
        app.build.status = status;
        assert_eq!(app.active_worker_count(), Some(0));
    }
    let _ = update(
        &mut app,
        Action::BuildRequested {
            target: Some("next".into()),
        },
    );
    assert_eq!(app.active_worker_count(), None);
    let _ = update(&mut app, Action::BuildStarted);
    assert_eq!(app.active_worker_count(), None);
}
