use super::*;

#[test]
fn next_generation_mouse_selects_exact_tasks_and_tabs() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Tasks;
    for index in 0..3 {
        let id = TaskId(format!("task-{index}"));
        app.tasks.insert(
            id.clone(),
            TaskInfo::active(id, format!("recipe-{index}"), "do_compile".into()),
        );
    }
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 7,
            },
            &app,
            160,
            48,
        ),
        Some(Action::ScrollBuildTasks { delta: 1 })
    );

    app.screen = Screen::Testing;
    let comparison = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 48,
            row: 3,
        },
        &app,
        160,
        48,
    );
    assert_eq!(
        comparison,
        Some(Action::SelectTestView(TestWorkspaceView::Comparison))
    );
    let _ = yoctui_model::update(&mut app, comparison.unwrap());
    assert_eq!(app.test_view, TestWorkspaceView::Comparison);
    app.screen = Screen::Security;
    app.security.view = SecurityView::Cves;
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 32,
                row: 3,
            },
            &app,
            160,
            48,
        ),
        Some(Action::Security(SecurityAction::CycleView))
    );
}
