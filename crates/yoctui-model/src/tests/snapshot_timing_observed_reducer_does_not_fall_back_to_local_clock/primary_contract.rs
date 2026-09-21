#[test]
fn snapshot_timing_observed_reducer_does_not_fall_back_to_local_clock() {
    use super::*;
    let mut app = App::new(64, 64 * 1024);
    let task = TaskInfo {
        id: TaskId("llvm-native:do_compile".into()),
        recipe: "llvm-native".into(),
        task: "do_compile".into(),
        ..TaskInfo::default()
    };
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![TaskEvent::ObservedStarted(task.clone())]),
    );
    assert_eq!(app.tasks[&task.id].started, None);
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![TaskEvent::ObservedCompleted {
            id: task.id.clone(),
            success: true,
            timing: ObservedTaskTiming {
                started: Some(SystemTime::UNIX_EPOCH),
                finished: None,
            },
        }]),
    );
    assert_eq!(
        app.completed_tasks[0].task.elapsed_at(SystemTime::now()),
        None
    );
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![TaskEvent::ObservedCompleted {
            id: task.id,
            success: true,
            timing: ObservedTaskTiming {
                started: Some(SystemTime::UNIX_EPOCH),
                finished: Some(SystemTime::now()),
            },
        }]),
    );
    assert_eq!(app.completed_tasks.len(), 1);
    assert_eq!(app.completed_tasks[0].task.finished, None);
}
