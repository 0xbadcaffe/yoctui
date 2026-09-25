use super::*;

#[test]
fn animation_is_visible_only_indeterminate_and_nonterminal() {
    let mut app = App::new(16, 16 * 1024);
    app.build.status = BuildStatus::Running;
    assert!(has_visible_indeterminate_activity(&app));

    app.screen = Screen::Recipes;
    assert!(!has_visible_indeterminate_activity(&app));

    app.screen = Screen::Tasks;
    app.build.total = Some(100);
    assert!(!has_visible_indeterminate_activity(&app));

    let task = TaskInfo::active(TaskId("busy".into()), "busybox".into(), "do_compile".into());
    app.tasks.insert(task.id.clone(), task);
    assert!(has_visible_indeterminate_activity(&app));

    app.build.status = BuildStatus::Completed;
    assert!(!has_visible_indeterminate_activity(&app));

    app.screen = Screen::Recipes;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Connecting;
    assert!(has_visible_indeterminate_activity(&app));
}
