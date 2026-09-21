use super::*;

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
