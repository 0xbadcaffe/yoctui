use super::*;
use crate::{Action, ObservedTaskTiming, TaskEvent, update};

#[test]
fn timeline_marks_a_deterministic_longest_dependency_path() {
    let mut app = App::new(16, 4096);
    let start = UNIX_EPOCH + Duration::from_secs(10);
    for (id, seconds, dependencies) in [
        ("a", 2, vec![]),
        ("b", 5, vec![TaskId("a".into())]),
        ("c", 1, vec![TaskId("a".into())]),
    ] {
        app.tasks.insert(
            TaskId(id.into()),
            TaskInfo {
                id: TaskId(id.into()),
                recipe: id.into(),
                task: "do_build".into(),
                state: TaskState::Completed,
                started: Some(start),
                finished: Some(start + Duration::from_secs(seconds)),
                dependencies,
                ..TaskInfo::default()
            },
        );
    }
    let rows = app.overview_timeline(start + Duration::from_secs(8));
    assert!(rows.iter().find(|row| row.id == "b").unwrap().critical);
    assert!(!rows.iter().find(|row| row.id == "c").unwrap().critical);
}

#[test]
fn timeline_retains_observed_tasks_after_completion_without_inventing_times() {
    let mut app = App::new(16, 4096);
    let started = UNIX_EPOCH + Duration::from_secs(10);
    let id = TaskId("iw:do_compile".into());
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![TaskEvent::ObservedStarted(TaskInfo {
            id: id.clone(),
            recipe: "iw".into(),
            task: "do_compile".into(),
            started: Some(started),
            ..TaskInfo::default()
        })]),
    );
    assert_eq!(
        app.overview_timeline(started + Duration::from_secs(2))[0].duration_millis,
        2000
    );
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![TaskEvent::ObservedCompleted {
            id,
            success: true,
            timing: ObservedTaskTiming {
                started: Some(started),
                finished: Some(started + Duration::from_secs(3)),
            },
        }]),
    );
    assert!(app.tasks.is_empty());
    let rows = app.overview_timeline(started + Duration::from_secs(100));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].duration_millis, 3000);
    assert!(rows[0].timing_available);
    assert_eq!(rows[0].state, TaskState::Completed);
    app.completed_tasks[0].task.started = None;
    let unknown = app.overview_timeline(started + Duration::from_secs(100));
    assert!(!unknown[0].timing_available);
    assert!(!unknown[0].critical);
}
