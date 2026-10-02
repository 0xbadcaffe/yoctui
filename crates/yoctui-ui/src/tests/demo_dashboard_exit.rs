use super::*;

fn narrow_dashboard(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(52, 16)).unwrap();
    terminal
        .draw(|frame| dashboard_render::dashboard(frame, app, frame.area(), literal_now()))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn demo_dashboard_exit_keeps_observed_status_and_exact_codes_after_resize() {
    let mut app = App::new(32, 8192);
    app.require_daemon = true;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.build.target = Some("a-very-long-observed-current-image-target".into());
    for (status, code) in [
        (yoctui_model::BuildStatus::Failed, 1),
        (yoctui_model::BuildStatus::Completed, 0),
        (yoctui_model::BuildStatus::Cancelled, -255),
        (yoctui_model::BuildStatus::Failed, i32::MAX),
    ] {
        app.build.status = status;
        app.build.exit_code = Some(code);
        for (width, height) in [(100, 25), (110, 26), (160, 50), (200, 60)] {
            let output = rendered_text_at(&app, width, height, literal_now());
            assert!(
                output.contains(&format!("Exit code: {code}")),
                "{width}x{height}: {output}"
            );
            assert!(
                output.contains(&status.to_string()),
                "{width}x{height}: {output}"
            );
        }
        let narrow = narrow_dashboard(&app);
        assert!(narrow.contains(&format!("Exit code: {code}")), "{narrow}");
        assert!(narrow.contains(&status.to_string()), "{narrow}");
        assert_eq!(app.build.exit_code, Some(code));
        assert_eq!(app.build.status, status);
    }
}

#[test]
fn demo_dashboard_exit_does_not_infer_missing_or_offline_current_outcomes() {
    let mut app = App::new(32, 8192);
    app.require_daemon = true;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.build.status = yoctui_model::BuildStatus::Idle;
    app.build_history.push_back(yoctui_model::BuildRecord {
        target: Some("old-image".into()),
        success: true,
        exit_code: Some(0),
        completed_tasks: 1,
        elapsed: None,
        warnings: 0,
        errors: 0,
    });
    for (width, height) in [(100, 25), (160, 50)] {
        let output = rendered_text_at(&app, width, height, literal_now());
        assert!(output.contains("Exit code: none"), "{output}");
        assert!(!output.contains("Exit code: 0"), "{output}");
    }
    assert!(narrow_dashboard(&app).contains("Exit code: none"));
    app.build.status = yoctui_model::BuildStatus::Failed;
    app.build.exit_code = Some(73);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Disconnected;
    for (width, height) in [(100, 25), (160, 50)] {
        let output = rendered_text_at(&app, width, height, literal_now());
        assert!(output.contains("Exit code: unavailable"), "{output}");
        assert!(output.contains("Offline"), "{output}");
        assert!(!output.contains("Exit code: 73"), "{output}");
        assert!(!output.contains("Exit code: 0"), "{output}");
    }
    let narrow = narrow_dashboard(&app);
    assert!(
        narrow.contains("Exit code: unavailable") && narrow.contains("Offline"),
        "{narrow}"
    );
    assert!(!narrow.contains("Exit code: 73"), "{narrow}");
    let navigator = rendered_text_at(&app, 80, 24, literal_now());
    assert!(navigator.contains("Panes: [Navigator]"), "{navigator}");
    assert!(!navigator.contains("Build Overview"), "{navigator}");
    assert_eq!(app.build.exit_code, Some(73));
    assert_eq!(app.build_history.len(), 1);
}
