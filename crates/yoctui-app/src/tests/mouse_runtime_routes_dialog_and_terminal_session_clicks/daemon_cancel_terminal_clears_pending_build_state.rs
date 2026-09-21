use super::*;

#[test]
fn daemon_cancel_terminal_clears_pending_build_state() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.build.status = yoctui_model::BuildStatus::Cancelling;
    apply_daemon_build_event(
        &mut app,
        yoctui_protocol::daemon::DaemonBuildEvent::Completed {
            success: false,
            exit_code: Some(130),
            finished_unix_ms: None,
        },
    );
    assert_eq!(app.build.status, yoctui_model::BuildStatus::Cancelled);
    assert_eq!(app.build.exit_code, Some(130));
}
