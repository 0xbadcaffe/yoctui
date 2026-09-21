use super::*;

#[test]
fn dashboard_navigator_focus_always_skips_read_only_panes() {
    let mut app = App::new(32, 8_192);
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(app.focus, FocusTarget::Navigator);
    let _ = update(&mut app, Action::Focus(FocusTarget::Navigator));
    assert_eq!(app.focus, FocusTarget::Navigator);

    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Navigator);

    app.tasks.insert(
        crate::TaskId("busybox:do_compile".into()),
        crate::TaskInfo::active(
            crate::TaskId("busybox:do_compile".into()),
            "busybox".into(),
            "do_compile".into(),
        ),
    );
    let _ = update(&mut app, Action::CycleFocus { backwards: false });
    assert_eq!(app.focus, FocusTarget::Navigator);

    app.completed_tasks.push_back(crate::CompletedTask {
        task: crate::TaskInfo::active(
            crate::TaskId("bash:do_compile".into()),
            "bash".into(),
            "do_compile".into(),
        ),
        success: true,
    });
    let _ = update(&mut app, Action::CycleFocus { backwards: true });
    assert_eq!(app.focus, FocusTarget::Navigator);
    let _ = update(&mut app, Action::Focus(FocusTarget::Workspace));
    assert_eq!(app.focus, FocusTarget::Navigator);

    app.navigator_selection = 0;
    let _ = update(&mut app, Action::ActivateNavigator);
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(app.focus, FocusTarget::Navigator);
}
