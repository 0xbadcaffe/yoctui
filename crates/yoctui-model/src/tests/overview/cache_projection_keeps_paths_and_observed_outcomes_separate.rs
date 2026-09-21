use super::*;

#[test]
fn cache_projection_keeps_paths_and_observed_outcomes_separate() {
    let mut app = App::new(16, 4096);
    app.workspace = Workspace::default();
    app.workspace
        .variables
        .insert("SSTATE_DIR".into(), "/cache/sstate".into());
    app.tasks.insert(
        TaskId("s".into()),
        TaskInfo {
            id: TaskId("s".into()),
            task: "do_packagedata_setscene".into(),
            state: TaskState::Completed,
            ..TaskInfo::default()
        },
    );
    app.tasks.insert(
        TaskId("f".into()),
        TaskInfo {
            id: TaskId("f".into()),
            task: "do_fetch".into(),
            state: TaskState::Failed,
            ..TaskInfo::default()
        },
    );
    crate::update(
        &mut app,
        crate::Action::TaskCompleted {
            id: TaskId("s".into()),
            success: true,
        },
    );
    crate::update(
        &mut app,
        crate::Action::TaskCompleted {
            id: TaskId("f".into()),
            success: false,
        },
    );
    let projection = app.overview_cache();
    assert_eq!(projection.sstate_hits, 1);
    assert_eq!(projection.fetch_failed, 1);
    assert_eq!(projection.sstate_dir.as_deref(), Some("/cache/sstate"));
    assert_eq!(projection.downloads_dir, None);
}
