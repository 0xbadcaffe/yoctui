use super::*;

#[test]
fn completed_progress_dashboard_and_tasks_are_stable_without_invented_percentage() {
    for screen in [Screen::Dashboard, Screen::Tasks] {
        for total in [None, Some(0)] {
            let mut app = App::new(8, 1024);
            app.onboarding.open = false;
            app.screen = screen;
            app.build.status = BuildStatus::Completed;
            app.build.exit_code = Some(0);
            app.build.total = total;
            app.build.target = Some("obmc-phosphor-image".into());
            let text = rendered_text(&app, 240, 50);
            assert!(text.contains("completed successfully"), "{text}");
            assert!(!text.contains("progress unknown"), "{text}");
            assert!(!text.contains("Overall  100%"), "{text}");
            assert!(
                text.contains(if total.is_some() {
                    "no tasks required"
                } else {
                    "total unavailable"
                }),
                "{text}"
            );
            for (w, h) in [(160, 50), (120, 32), (80, 24), (20, 5), (1, 1)] {
                let _ = rendered_text(&app, w, h);
            }
        }
    }
}

#[test]
fn completed_progress_preserves_live_unknown_failure_cancellation_and_known_gauges() {
    let mut app = App::new(8, 1024);
    app.onboarding.open = false;
    app.screen = Screen::Tasks;
    for status in [
        BuildStatus::Running,
        BuildStatus::Failed,
        BuildStatus::Cancelled,
        BuildStatus::Lost,
    ] {
        app.build.status = status;
        let text = rendered_text(&app, 240, 50);
        assert!(text.contains("progress unknown"), "{text}");
        assert!(!text.contains("completed successfully"));
    }
    app.build.status = BuildStatus::Completed;
    app.build.exit_code = Some(0);
    app.build.completed = 7;
    app.build.total = Some(7);
    assert!(rendered_text(&app, 240, 50).contains("Overall  100%  7/7"));
    app.build.total = None;
    assert!(rendered_text(&app, 240, 50).contains("7 observed · total unavailable"));
    app.build.total = Some(0);
    assert!(rendered_text(&app, 240, 50).contains("7 observed · reported total 0"));
}
