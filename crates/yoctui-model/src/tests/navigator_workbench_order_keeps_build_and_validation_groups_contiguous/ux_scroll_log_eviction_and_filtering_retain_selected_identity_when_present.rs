use super::*;

#[test]
fn ux_scroll_log_eviction_and_filtering_retain_selected_identity_when_present() {
    let mut navigator = App::new(8, 1_000);
    let _ = update(
        &mut navigator,
        Action::SelectNavigator { delta: isize::MAX },
    );
    assert_eq!(
        navigator.navigator_selection,
        NAVIGATOR_SCREENS.len() - 1,
        "edge navigation must not iterate once per signed delta"
    );
    let _ = update(
        &mut navigator,
        Action::SelectNavigator { delta: isize::MIN },
    );
    assert_eq!(navigator.navigator_selection, 0);

    let mut tasks = App::new(8, 1_000);
    tasks.screen = Screen::Tasks;
    for index in 0..15 {
        let id = TaskId(format!("scroll-task-{index}"));
        tasks.tasks.insert(
            id.clone(),
            TaskInfo::active(id, "busybox".into(), "do_compile".into()),
        );
    }
    let _ = update(&mut tasks, Action::ScrollCurrent { to_end: true });
    assert_eq!(tasks.task_progress_scroll, 14);
    let _ = update(&mut tasks, Action::ScrollCurrent { to_end: false });
    assert_eq!(tasks.task_progress_scroll, 0);

    let mut logs = LogState::new(3, 1_000);
    logs.insert(log("alpha"));
    logs.insert(log("beta"));
    logs.insert(log("gamma"));
    logs.follow = false;
    logs.paused_len = Some(3);
    logs.selection = 1;
    let beta_id = logs.selected().unwrap().id;

    logs.insert(log("delta"));
    assert_eq!(logs.selected().map(|entry| entry.id), Some(beta_id));
    assert_eq!(
        logs.selection, 0,
        "eviction before the row shifts its index"
    );

    let mut app = App::new(8, 1_000);
    app.logs.insert(log("alpha"));
    app.logs.insert(log("beta match"));
    app.logs.follow = false;
    app.logs.paused_len = Some(2);
    app.logs.selection = 1;
    let beta_id = app.logs.selected().unwrap().id;
    let _ = update(&mut app, Action::BeginLogSearch);
    let _ = update(&mut app, Action::AppendLogQuery('b'));
    assert_eq!(app.logs.selected().map(|entry| entry.id), Some(beta_id));
    assert_eq!(
        app.logs.selection, 0,
        "filtering recomputes the stable row index"
    );
}
