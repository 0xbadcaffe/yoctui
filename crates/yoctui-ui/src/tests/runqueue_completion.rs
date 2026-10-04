use super::*;
use yoctui_model::{TaskEvent, TaskId, TaskInfo};

#[test]
fn runqueue_completion_renders_noexec_success_at_responsive_sizes() {
    let mut app = App::new(16, 4096);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    let id = TaskId("obmc-phosphor-image:do_build".into());
    update(
        &mut app,
        Action::TaskEvents(vec![
            TaskEvent::Queued(TaskInfo {
                id: id.clone(),
                recipe: "obmc-phosphor-image".into(),
                task: "do_build".into(),
                ..TaskInfo::default()
            }),
            TaskEvent::Completed { id, success: true },
        ]),
    );
    for (width, height) in [(160, 50), (120, 40), (96, 32), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("do_build"), "{width}x{height}: {text}");
        assert!(text.contains("Succeeded"), "{width}x{height}: {text}");
        assert!(!text.contains("? Lost"), "{width}x{height}: {text}");
    }
    // Too-small terminals still degrade safely rather than rendering a task pane.
    let text = rendered_text(&app, 40, 12);
    assert!(!text.contains("do_build"));
}
