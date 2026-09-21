use super::*;

#[test]
fn ux_logs_tasks_errors_and_job_history_open_exact_correlated_records() {
    let mut tasks = App::new(20, 4_000);
    tasks.screen = Screen::Tasks;
    let task = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    let _ = update(&mut tasks, Action::TaskStarted(task));
    let _ = update(&mut tasks, Action::Log(log("unrelated")));
    let correlated = tagged_log(
        "busybox",
        "do_compile",
        Severity::Warning,
        "exact task warning",
    );
    let _ = update(&mut tasks, Action::Log(correlated));
    let expected = tasks.logs.entries.back().unwrap().id;
    let _ = update(&mut tasks, Action::Open(Screen::Logs));
    assert_eq!(tasks.logs.selected().map(|entry| entry.id), Some(expected));
    assert!(!tasks.logs.follow);

    tasks.error_selection = 0;
    tasks.screen = Screen::Errors;
    let _ = update(&mut tasks, Action::JumpToSelectedError);
    assert_eq!(tasks.logs.selected().map(|entry| entry.id), Some(expected));

    let mut history = App::new(20, 4_000);
    history.screen = Screen::BuildHistory;
    history.build_history.push_back(BuildRecord {
        target: Some("core-image-minimal".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(1)),
        completed_tasks: 1,
        warnings: 0,
        errors: 0,
    });
    let mut entry = log("matching build output");
    entry.build = Some("core-image-minimal".into());
    history.logs.insert(entry);
    let expected = history.logs.entries.back().unwrap().id;
    let _ = update(&mut history, Action::Open(Screen::Logs));
    assert_eq!(
        history.logs.selected().map(|entry| entry.id),
        Some(expected)
    );
}
